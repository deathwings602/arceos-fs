use core::mem::size_of;
use log::*;

use super::{
    DiskInode, 
    Ext2FileSystem, layout::{
        MAX_NAME_LEN, DirEntryHead, EXT2_FT_UNKNOWN, EXT2_FT_DIR, EXT2_FT_REG_FILE,
        DEFAULT_IMODE, EXT2_S_IFDIR, EXT2_S_IFLNK, IMODE
    }
};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    inode_id: usize,
    block_id: usize,
    block_offset: usize,
    fs: Arc<Ext2FileSystem>,
    file_type: u8
}

impl Inode {
    pub fn new(
        inode_id: usize,
        block_id: usize,
        block_offset: usize,
        fs: Arc<Ext2FileSystem>
    ) -> Self {
        let mut inode = Self {
            inode_id,
            block_id,
            block_offset,
            fs,
            file_type: EXT2_FT_UNKNOWN
        };
        inode.read_file_type();
        inode
    }

    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        let inode_block = self.fs.manager.lock().get_block_cache(self.block_id);
        let ret = inode_block.lock()
            .read(self.block_offset, f);
        ret
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        let inode_block = self.fs.manager.lock().get_block_cache(self.block_id);
        let ret = inode_block.lock()
            .modify(self.block_offset, f);
        ret
    }

    pub fn read_file_type(&mut self) {
        if self.file_type == EXT2_FT_UNKNOWN {
            self.file_type = self.read_disk_inode(|disk_inode| {
                disk_inode.file_code()
            });
        }
    }

    pub fn file_type(&self) -> u8 {
        if self.file_type == EXT2_FT_UNKNOWN {
            self.read_disk_inode(|disk_inode| {
                disk_inode.file_code()
            })
        } else {
            self.file_type
        }
    }

    /// Find inode under a disk inode by name (DirEntry, pos, prev_offset)
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<(DirEntryHead, usize, usize)> {
        debug!("find_inode_id");
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let mut buffer = [0 as u8; MAX_NAME_LEN];
        let mut dir_entry_head = DirEntryHead::empty();
        let mut offset: usize = 0;
        let mut pos: usize = 0;
        let mut prev_offset: usize = 0;

        while offset + size_of::<DirEntryHead>() < disk_inode.i_size as usize {
            assert_eq!(disk_inode.read_at(offset, dir_entry_head.as_bytes_mut(), &self.fs.manager),
                        size_of::<DirEntryHead>());
            let name_len = dir_entry_head.name_len as usize;
            let name_buffer = &mut buffer[0..name_len];
            let name_offset = offset + size_of::<DirEntryHead>();
            assert_eq!(disk_inode.read_at(name_offset, name_buffer, &self.fs.manager),
                        name_len);
            if name_buffer == name.as_bytes() {
                return Some((dir_entry_head, pos, prev_offset));
            }
            prev_offset = offset;
            offset += dir_entry_head.rec_len as usize;
            pos += 1;
        }

        None
    }

    pub fn get_inode_id(&self, name: &str) -> Option<(DirEntryHead, usize, usize)> {
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode)
        })
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        if let Some(de) = self.get_inode_id(name)
                                .map(|(de, _, _)| {
                                    de
                                })
        {
            let (block_id, block_offset) = self.fs.get_disk_inode_pos(de.inode);
            let mut inode = Self {
                inode_id: de.inode as usize,
                block_id: block_id as usize,
                block_offset,
                fs: self.fs.clone(),
                file_type: de.file_type
            };
            inode.read_file_type();
            Some(Arc::new(inode))
        } else {
            None
        }
    }

    pub fn create(&self, name: &str, mut file_type: u16) -> Option<Arc<Inode>> {
        if self.file_type() != EXT2_FT_DIR {
            error!("Try to create a file under a file");
            return None;
        }
        if self.get_inode_id(name).is_some() {
            error!("Try to create a file already exists");
            return None;
        }
        file_type &= 0xF000;
        let new_inode_id = self.fs.alloc_inode().unwrap();
        let (new_inode_block_id, new_inode_block_offset) = self.fs.get_disk_inode_pos(new_inode_id);
        let inode_block = self.fs.manager.lock().get_block_cache(new_inode_block_id as _);
        inode_block.lock()
            .modify(new_inode_block_offset, |disk_inode: &mut DiskInode| {
                *disk_inode = DiskInode::new(DEFAULT_IMODE, file_type, 0, 0);
                let cur_time = self.fs.timer.get_current_time();
                disk_inode.i_atime = cur_time;
                disk_inode.i_ctime = cur_time;
            });

        let mut new_inode = Ext2FileSystem::get_inode(&self.fs, new_inode_id as usize).unwrap();
        new_inode.read_file_type();
        self.append_dir_entry(new_inode_id as usize, name, new_inode.file_type);

        if file_type == EXT2_S_IFDIR {
            new_inode.link(".", new_inode_id as usize);
            new_inode.link("..", self.inode_id);
        }

        self.fs.write_meta();
        Some(Arc::new(new_inode))
    }

    pub fn link(&self, name: &str, inode_id: usize) -> bool {
        debug!("link {} to {}", name, inode_id);
        if inode_id == 0 {
            return false;
        }
        if let Some(mut inode) = Ext2FileSystem::get_inode(&self.fs, inode_id) {
            if self.find(name).is_some() {
                // already exists
                false
            } else {
                inode.read_file_type();
                if inode.file_type() != EXT2_FT_REG_FILE {
                    return false;
                }
                self.append_dir_entry(inode.inode_id, name, inode.file_type);
                inode.increase_nlink(1);
                self.fs.write_meta();
                true
            }
        } else {
            false
        }
    }

    pub fn symlink(&self, name: &str, path_name: &str) -> bool {
        debug!("symlink {} to {}", name, path_name);
        if let Some(inode) = self.create(name, EXT2_S_IFLNK) {
            inode.append(path_name.as_bytes());
            true
        } else {
            false
        }
    }

    fn ls_disk(&self, disk_inode: &DiskInode) -> Vec<String> {
        assert!(disk_inode.is_dir());
        let mut buffer = [0 as u8; MAX_NAME_LEN];
        let mut names: Vec<String> = Vec::new();

        let mut dir_entry_head = DirEntryHead::empty();
        let mut offset: usize = 0;

        while offset + size_of::<DirEntryHead>() < disk_inode.i_size as usize {
            assert_eq!(disk_inode.read_at(offset, dir_entry_head.as_bytes_mut(), &self.fs.manager),
                        size_of::<DirEntryHead>());
            let name_len = dir_entry_head.name_len as usize;
            let name_buffer = &mut buffer[0..name_len];
            let name_offset = offset + size_of::<DirEntryHead>();
            assert_eq!(disk_inode.read_at(name_offset, name_buffer, &self.fs.manager),
                        name_len);
            names.push(String::from_utf8_lossy(name_buffer).to_string());
            offset += dir_entry_head.rec_len as usize;
        };

        names
    }

    pub fn ls(&self) -> Vec<String> {
        self.read_disk_inode(|disk_inode| {
            self.ls_disk(disk_inode)
        })
    }

    fn unlink_below(&self) {
        if self.file_type() != EXT2_FT_DIR {
            return;
        }
        let names = self.ls();

        for file_name in names.iter() {
            if file_name.as_str() == "." || file_name.as_str() == ".." {
                // special case
                continue;
            }
            let child_inode = self.find(file_name.as_str()).unwrap();
            child_inode.unlink_below();
            self.unlink_single(file_name.as_str());
        }

        // .
        self.decrease_nlink(1);
        // ..
        let parent_inode = self.find("..").unwrap();
        parent_inode.decrease_nlink(1);
    }

    /// unlink recursively
    pub fn unlink(&self, name: &str) -> bool {
        debug!("unlink {}", name);
        if self.file_type() != EXT2_FT_DIR {
            return false;
        }
        if name == "." || name == ".." {
            error!("Can not unlink . or ..");
            return false;
        }

        if let Some(inode) = self.find(name) {
            inode.unlink_below();
            self.unlink_single(name);
            true
        } else {
            false
        }
    }

    // ----- ACL ------
    pub fn chown(&self, uid: Option<usize>, gid: Option<usize>) {
        self.modify_disk_inode(|disk_inode| {
            if let Some(uid) = uid {
                disk_inode.i_uid = uid as _;
            }
            if let Some(gid) = gid {
                disk_inode.i_gid = gid as _;
            }
            disk_inode.i_mtime = self.fs.timer.get_current_time();
        })
    }
    pub fn chmod(&self, access: IMODE) {
        self.modify_disk_inode(|disk_inode| {
            disk_inode.i_mode = (disk_inode.i_mode & 0o7000) | access.bits();
            let cur_time = self.fs.timer.get_current_time();
            disk_inode.i_ctime = cur_time;
            disk_inode.i_atime = cur_time;
        });
    }

    // ----- Basic operation -----
    pub fn ftruncate(&self, new_size: u32) -> bool {
        if self.file_type() != EXT2_FT_REG_FILE {
            return false;
        }
        self.modify_disk_inode(|disk_inode| {
            if disk_inode.i_size > new_size {
                self.decrease_size(new_size, disk_inode);
            } else {
                self.increase_size(new_size, disk_inode);
            }
            let cur_time = self.fs.timer.get_current_time();
            disk_inode.i_ctime = cur_time;
        });
        true
    }

    fn unlink_single(&self, name: &str) -> bool {
        if self.file_type() != EXT2_FT_DIR {
            return false;
        }
        if name == "." || name == ".." {
            return false;
        }
        if let Some((de, pos, prev_offset)) = self.get_inode_id(name) {
            assert!(pos != 0);
            let mut buf = [0 as u8; size_of::<DirEntryHead>()];
            self.read_at(prev_offset, &mut buf);
            unsafe {
                (*(&mut buf as *mut u8 as *mut DirEntryHead)).rec_len += de.rec_len;
            }
            self.write_at(prev_offset, &buf);
            
            let target_inode = Ext2FileSystem::get_inode(&self.fs, de.inode as usize).unwrap();
            target_inode.decrease_nlink(1);
            true
        } else {
            false
        }
    }

    fn decrease_nlink(&self, by: usize) {
        let mut clean = false;
        self.modify_disk_inode(|disk_inode| {
            assert!(disk_inode.i_links_count >= by as u16);
            disk_inode.i_links_count -= by as u16;
            clean = disk_inode.i_links_count == 0;
        });

        if clean {
            self.clear();
            self.fs.dealloc_inode(self.inode_id as u32);
        }
    }

    fn increase_nlink(&self, by: usize) {
        self.modify_disk_inode(|disk_inode| {
            disk_inode.i_links_count += by as u16;
        });
    }

    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
    ) {
        if new_size < disk_inode.i_size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let new_blocks = self.fs.batch_alloc_data(blocks_needed as _);
        assert!(new_blocks.len() == blocks_needed as _);
        disk_inode.increase_size(new_size, new_blocks, &self.fs.manager);
    }

    /// Decrease the size of a disk node
    fn decrease_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
    ) {
        if new_size >= disk_inode.i_size {
            return;
        }
        let blocks_unused = disk_inode.decrease_size(new_size, &self.fs.manager);
        self.fs.batch_dealloc_block(&blocks_unused);
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        self.modify_disk_inode(|disk_inode| {
            let blocks = disk_inode.i_blocks;
            let data_blocks_dealloc = disk_inode.clear_size(&self.fs.manager);
            if data_blocks_dealloc.len() != DiskInode::total_blocks(blocks * 512) as usize {
                error!("clear: {} != {}", data_blocks_dealloc.len(), DiskInode::total_blocks(blocks * 512) as usize);
            }
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(blocks * 512) as usize);
            let cur_time = self.fs.timer.get_current_time();
            disk_inode.i_atime = cur_time;
            disk_inode.i_mtime = cur_time;
            self.fs.batch_dealloc_block(&data_blocks_dealloc);
        });
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.modify_disk_inode(|disk_inode| {
            let cur_time = self.fs.timer.get_current_time();
            disk_inode.i_atime = cur_time;
            disk_inode.read_at(offset, buf, &self.fs.manager)
        })
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode);
            let cur_time = self.fs.timer.get_current_time();
            disk_inode.i_atime = cur_time;
            disk_inode.i_mtime = cur_time;
            disk_inode.write_at(offset, buf, &self.fs.manager)
        });
        size
    }
    /// Write data at the end of file
    pub fn append(&self, buf: &[u8]) -> usize {
        let size = self.modify_disk_inode(|disk_inode| {
            let origin_size = disk_inode.i_size as usize;
            self.increase_size((origin_size + buf.len()) as u32, disk_inode);
            let cur_time = self.fs.timer.get_current_time();
            disk_inode.i_atime = cur_time;
            disk_inode.i_mtime = cur_time;
            disk_inode.write_at(origin_size, buf, &self.fs.manager)
        });
        size
    }
    fn append_dir_entry(&self, inode: usize, name: &str, file_type: u8) {
        let dir_entry = DirEntryHead::create(inode, name, file_type);
        self.append(dir_entry.as_bytes());
        let name_len = name.as_bytes().len();
        self.append(&name.as_bytes()[0..name_len.min(MAX_NAME_LEN)]);
    }
}