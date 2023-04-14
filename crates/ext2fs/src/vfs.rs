#![allow(unused)]
use core::mem::size_of;

use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DiskInode, 
    Ext2FileSystem, config::BLOCK_SIZE, layout::{
        MAX_NAME_LEN, DirEntryHead, EXT2_FT_UNKNOWN, EXT2_FT_DIR, EXT2_FT_REG_FILE,
        DEFAULT_IMODE, EXT2_S_IFDIR
    }
};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    inode_id: usize,
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<Ext2FileSystem>>,
    block_device: Arc<dyn BlockDevice>,
    file_type: u8
}

impl Inode {
    pub fn new(
        inode_id: usize,
        block_id: usize,
        block_offset: usize,
        fs: Arc<Mutex<Ext2FileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        let mut inode = Self {
            inode_id,
            block_id,
            block_offset,
            fs,
            block_device,
            file_type: EXT2_FT_UNKNOWN
        };
        inode.read_file_type();
        inode
    }

    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
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
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let mut buffer = [0 as u8; MAX_NAME_LEN];
        let mut dir_entry_head = DirEntryHead::empty();
        let mut offset: usize = 0;
        let mut pos: usize = 0;
        let mut prev_offset: usize = 0;

        while offset + size_of::<DirEntryHead>() < disk_inode.i_size as usize {
            assert_eq!(disk_inode.read_at(offset, dir_entry_head.as_bytes_mut(), &self.block_device),
                        size_of::<DirEntryHead>());
            let name_len = dir_entry_head.name_len as usize;
            let name_buffer = &mut buffer[0..name_len];
            let mut name_offset = offset + size_of::<DirEntryHead>();
            assert_eq!(disk_inode.read_at(name_offset, name_buffer, &self.block_device),
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
        let fs = self.fs.lock();
        if let Some(de) = self.get_inode_id(name)
                                .map(|(de, _, _)| {
                                    de
                                })
        {
            let (block_id, block_offset) = fs.get_disk_inode_pos(de.inode);
            Some(Arc::new(Self {
                inode_id: de.inode as usize,
                block_id: block_id as usize,
                block_offset,
                fs: self.fs.clone(),
                block_device: self.block_device.clone(),
                file_type: de.file_type
            }))
        } else {
            None
        }
    }

    pub fn create(&self, name: &str, mut file_type: u16) -> Option<Arc<Inode>> {
        if self.file_type() != EXT2_FT_DIR {
            return None;
        }
        if self.get_inode_id(name).is_some() {
            return  None;
        }
        file_type &= 0xE000;
        let mut fs = self.fs.lock();
        let new_inode_id = fs.alloc_inode().unwrap();
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |disk_inode: &mut DiskInode| {
                *disk_inode = DiskInode::new(DEFAULT_IMODE, file_type, 0, 0);
            });
        
        drop(fs);

        let mut new_inode = Ext2FileSystem::get_inode(&self.fs, new_inode_id as usize).unwrap();
        new_inode.read_file_type();
        self.append_dir_entry(new_inode_id as usize, name, new_inode.file_type);

        if file_type == EXT2_S_IFDIR {
            new_inode.link(".", new_inode_id as usize);
            new_inode.link("..", self.inode_id);
        }

        Some(Arc::new(new_inode))
    }

    pub fn link(&self, name: &str, inode_id: usize) -> bool {
        if inode_id == 0 {
            return false;
        }
        if let Some(mut inode) = Ext2FileSystem::get_inode(&self.fs, inode_id) {
            if self.find(name).is_some() {
                // already exists
                false
            } else {
                inode.read_file_type();
                self.append_dir_entry(inode.inode_id, name, inode.file_type);
                inode.increase_nlink(1);
                true
            }
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
            assert_eq!(disk_inode.read_at(offset, dir_entry_head.as_bytes_mut(), &self.block_device),
                        size_of::<DirEntryHead>());
            let name_len = dir_entry_head.name_len as usize;
            let name_buffer = &mut buffer[0..name_len];
            let mut name_offset = offset + size_of::<DirEntryHead>();
            assert_eq!(disk_inode.read_at(name_offset, name_buffer, &self.block_device),
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
        if self.file_type() != EXT2_FT_DIR {
            return false;
        }
        if name == "." || name == ".." {
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
            self.fs.lock().dealloc_inode(self.inode_id as u32);
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
        fs: &mut MutexGuard<Ext2FileSystem>,
    ) {
        if new_size < disk_inode.i_size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data().unwrap());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// Decrease the size of a disk node
    fn decrease_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<Ext2FileSystem>,
    ) {
        if new_size >= disk_inode.i_size {
            return;
        }
        let blocks_unused = disk_inode.decrease_size(new_size, &self.block_device);
        for block in blocks_unused.iter() {
            fs.dealloc_block(*block);
        }
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let blocks = disk_inode.i_blocks;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(blocks * 512) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_block(data_block);
            }
        });
        block_cache_sync_all();
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Write data at the end of file
    pub fn append(&self, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            let origin_size = disk_inode.i_size as usize;
            self.increase_size((origin_size + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(origin_size, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    fn append_dir_entry(&self, inode: usize, name: &str, file_type: u8) {
        let dir_entry = DirEntryHead::create(inode, name, file_type);
        self.append(dir_entry.as_bytes());
        self.append(&name.as_bytes()[0..MAX_NAME_LEN]);
    }
}