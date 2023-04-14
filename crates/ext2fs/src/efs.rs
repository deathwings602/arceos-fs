#![allow(unused)]
use core::mem::size_of;

use super::{
    block_cache_sync_all, get_block_cache, Bitmap, BlockDevice, DiskInode, BlockGroupDesc, Inode,
    SuperBlock, config::{
        BLOCK_SIZE, BLOCKS_PER_GRP, RESERVED_BLOCKS_PER_GRP, EXT2_ROOT_INO,
        FIRST_DATA_BLOCK, INODES_PER_GRP, EXT2_GOOD_OLD_FIRST_INO, SUPER_BLOCK_OFFSET
    },
    layout::{IMODE, EXT2_S_IFDIR, EXT2_S_IFREG}
};
use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;

pub struct Ext2FileSystem {
    ///Real device
    pub block_device: Arc<dyn BlockDevice>,
    /// Super block cache
    pub super_block: SuperBlock,
    /// Group description
    pub group_desc_table: Vec<BlockGroupDesc>
}

type DataBlock = [u8; BLOCK_SIZE];

impl Ext2FileSystem {
    /// Create an ext2 file system in a device
    pub fn create(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        assert!(block_device.block_size() == BLOCK_SIZE, "Unsupported block size");
        
        let mut block_num = block_device.block_num();
        let mut group_num = (block_num + BLOCKS_PER_GRP - 1)/BLOCKS_PER_GRP;
        assert!(group_num >= 1, "Size is at least 32 MB");
        let mut last_group_block_num = block_num - (group_num - 1) * BLOCKS_PER_GRP;

        if last_group_block_num <= RESERVED_BLOCKS_PER_GRP {
            group_num -= 1;
            last_group_block_num = BLOCKS_PER_GRP;
        }
        assert!(group_num >= 1);
        block_num = (group_num - 1) * BLOCKS_PER_GRP + last_group_block_num;
        let group_desc_block_num = (block_num * size_of::<BlockGroupDesc>() + BLOCK_SIZE - 1)/BLOCK_SIZE;

        let mut group_desc_table:Vec<BlockGroupDesc> = Vec::new();
        for group_id in 0..group_num {
            let mut block_bitmap: usize = 0;
            let mut free_blocks: usize = 0;
            if group_id == 0 {
                block_bitmap = (FIRST_DATA_BLOCK + 1) + group_desc_block_num;
                free_blocks = if group_id == group_num - 1 {
                    last_group_block_num - block_bitmap - RESERVED_BLOCKS_PER_GRP
                } else {
                    BLOCKS_PER_GRP - block_bitmap - RESERVED_BLOCKS_PER_GRP
                }
                // first group
            } else if group_id == group_num - 1 {
                // last group
                block_bitmap = BLOCKS_PER_GRP * group_id;
                free_blocks = last_group_block_num - RESERVED_BLOCKS_PER_GRP;
            } else {
                block_bitmap = BLOCKS_PER_GRP * group_id;
                free_blocks = BLOCKS_PER_GRP - RESERVED_BLOCKS_PER_GRP;
            }
            group_desc_table.push(BlockGroupDesc::new(
                block_bitmap,
                block_bitmap + 1,
                block_bitmap + 2,
                free_blocks, INODES_PER_GRP, 0
            ));
        }

        let super_block = SuperBlock::new(
            INODES_PER_GRP * group_num,
            block_num,
            INODES_PER_GRP * group_num - EXT2_GOOD_OLD_FIRST_INO + 1,
            block_num - group_num * RESERVED_BLOCKS_PER_GRP - (FIRST_DATA_BLOCK + 1 + group_desc_block_num),
            group_num,
            "Image by hsh"
        );

        let mut fs = Self {
            block_device: block_device.clone(),
            super_block,
            group_desc_table
        };

        // clear all blocks except the first 1024 bytes
        for i in 0..block_num {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for (idx, byte) in data_block.iter_mut().enumerate() {
                        if i != 0 || idx >= 1024 {
                            *byte = 0;
                        }
                    }
                });
        }

        // TODO: mark reserved inodes and used data blocks
        fs.get_inode_bitmap(0)
            .range_alloc(&block_device, 1, EXT2_GOOD_OLD_FIRST_INO);
        for group_id in 0..group_num {
            fs.get_data_bitmap(group_id)
                .range_alloc(
                    &block_device, 
                    group_id * BLOCKS_PER_GRP, 
                    fs.group_desc_table[group_id].bg_block_bitmap as usize + RESERVED_BLOCKS_PER_GRP + 1
                );
            if group_id == group_num - 1 {
                fs.get_data_bitmap(group_id)
                    .range_alloc(
                        &block_device, 
                        last_group_block_num, 
                        (group_id + 1) * BLOCKS_PER_GRP
                    );
            }
        }
        fs.write_meta();
        block_cache_sync_all();

        // TODO: init '/' inode
        let (root_inode_block_id, root_inode_offset) = fs.get_disk_inode_pos(EXT2_ROOT_INO as u32);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                *disk_inode = DiskInode::new(
                    IMODE::from_bits_truncate(0x1FF), 
                    EXT2_S_IFREG, 0, 0);
            });

        // TODO: create dir entry '.' and '..' for '/'
        let fs = Arc::new(Mutex::new(fs));
        let root_inode = Self::root_inode(&fs);
        root_inode.link(".", EXT2_ROOT_INO);
        root_inode.link("..", EXT2_ROOT_INO);

        // TODO: write super blocks and group description table to disk
        fs.lock().write_meta();
        block_cache_sync_all();

        fs
    }

    /// Open a file system from disk
    pub fn open(block_device: Arc<dyn BlockDevice>, cur_time: u32) -> Arc<Mutex<Self>> {
        assert!(block_device.block_size() == BLOCK_SIZE, "Unsupported block size");

        let mut super_block = SuperBlock::empty();
        get_block_cache(FIRST_DATA_BLOCK, Arc::clone(&block_device))
            .lock()
            .read(SUPER_BLOCK_OFFSET, |sb: &SuperBlock| {
                super_block = *sb;
            });
        
        super_block.check_valid();

        let mut group_desc_table: Vec<BlockGroupDesc> = Vec::new();

        for group_id in 0..super_block.s_block_group_nr as usize {
            let block_id = super_block.s_first_data_block as usize + 1 + (group_id * size_of::<BlockGroupDesc>())/BLOCK_SIZE;
            let offset = (group_id * size_of::<BlockGroupDesc>())%BLOCK_SIZE;
            get_block_cache(block_id, Arc::clone(&block_device))
                .lock()
                .read(offset, |desc: &BlockGroupDesc| {
                    group_desc_table.push(*desc);
                });
        }

        let mut fs = Self {
            block_device: Arc::clone(&block_device),
            super_block,
            group_desc_table
        };

        fs.super_block.s_mnt_count += 1;
        fs.super_block.s_mtime = cur_time;

        fs.write_super_block();
        block_cache_sync_all();

        Arc::new(Mutex::new(fs))
    }

    /// Get root inode
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        Self::get_inode(efs, EXT2_ROOT_INO as usize).unwrap()
    }

    pub fn get_inode(efs: &Arc<Mutex<Self>>, inode_id: usize) -> Option<Inode> {
        let _fs = efs.lock();
        let block_device = Arc::clone(&_fs.block_device);
        if inode_id == 0 || inode_id > _fs.super_block.s_inodes_count as usize {
            None
        } else {
            let (block_id, offset) = _fs.get_disk_inode_pos(inode_id as u32);
            Some(Inode::new(
                inode_id,
                block_id as usize,
                offset,
                Arc::clone(efs),
                block_device
            ))
        }
    }

    /// Get inode block_id and offset from inode_id
    pub fn get_disk_inode_pos(&self, mut inode_id: u32) -> (u32, usize) {
        assert!(inode_id != 0); // invalid inode id
        inode_id -= 1;
        let group_id = inode_id/INODES_PER_GRP as u32;
        let group_offset = inode_id%INODES_PER_GRP as u32;
        let inode_size = size_of::<DiskInode>();
        let inode_per_block = BLOCK_SIZE/inode_size;
        let block_id = self.group_desc_table[group_id as usize].bg_inode_table + group_offset/inode_per_block as u32;

        (block_id, (group_offset as usize%inode_per_block) * inode_size)
    }

    /// Get inode bitmap for group x
    pub fn get_inode_bitmap(&self, group_id: usize) -> Bitmap {
        Bitmap::new(
            self.group_desc_table[group_id].bg_inode_bitmap as usize,
            group_id * INODES_PER_GRP + 1
        )
    }

    /// Get data bitmap for group x
    pub fn get_data_bitmap(&self, group_id: usize) -> Bitmap {
        Bitmap::new(
            self.group_desc_table[group_id].bg_block_bitmap as usize,
            group_id * BLOCKS_PER_GRP
        )
    }

    /// Allocate inode (will modify meta data)
    pub fn alloc_inode(&mut self) -> Option<u32> {
        for group_id in 0..self.group_desc_table.len() {
            if let Some(inode_id) = self.get_inode_bitmap(group_id).alloc(&self.block_device) {
                self.group_desc_table[group_id].bg_free_inodes_count -= 1; // still need to mantain bg_used_dir_count
                self.super_block.s_free_inodes_count -= 1;
                return  Some(inode_id as u32);
            }
        }
        None
    }

    /// Allocate data block (will modify meta data)
    pub fn alloc_data(&mut self) -> Option<u32> {
        for group_id in 0..self.group_desc_table.len() {
            if let Some(block_id) = self.get_data_bitmap(group_id).alloc(&self.block_device) {
                self.group_desc_table[group_id].bg_free_blocks_count -= 1; // still need to mantain bg_used_dir_count
                self.super_block.s_free_blocks_count -= 1;
                return  Some(block_id as u32);
            }
        }
        None
    }

    /// Dealloc inode (will modify meta data)
    pub fn dealloc_inode(&mut self, inode_id: u32) {
        assert!(inode_id != 0);
        let group_id = (inode_id as usize - 1) / INODES_PER_GRP;
        self.get_inode_bitmap(group_id).dealloc(&self.block_device, inode_id as usize);

        self.super_block.s_free_inodes_count += 1;
        self.group_desc_table[group_id].bg_free_inodes_count += 1;
    }

    /// Dealloc inode (will modify meta data)
    pub fn dealloc_block(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        
        let group_id = block_id as usize / BLOCKS_PER_GRP;
        self.get_data_bitmap(group_id).dealloc(&self.block_device, block_id as usize);
        self.super_block.s_free_blocks_count += 1;
        self.group_desc_table[group_id].bg_free_blocks_count += 1;
    }

    /// Write super block to disk
    pub fn write_super_block(&self) {
        let offset = if self.super_block.s_first_data_block == 0 { 1024 } else { 0 };
        get_block_cache(self.super_block.s_first_data_block as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(offset, |super_block: &mut SuperBlock| {
                *super_block = self.super_block;
            });
    }

    /// Write group description of group_id to disk
    pub fn write_group_desc(&self, group_id: usize) {
        let block_id = self.super_block.s_first_data_block as usize + 1 + (group_id * size_of::<BlockGroupDesc>())/BLOCK_SIZE;
        let offset = (group_id * size_of::<BlockGroupDesc>())%BLOCK_SIZE;
        get_block_cache(block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(offset, |desc: &mut BlockGroupDesc| {
                *desc = self.group_desc_table[group_id];
            });
    }

    /// Write all group description to disk
    pub fn write_all_group_desc(&self) {
        for group_id in 0..self.group_desc_table.len() {
            self.write_group_desc(group_id);
        }
    }

    /// Write all meta data to disk
    pub fn write_meta(&self) {
        self.write_super_block();
        self.write_all_group_desc();
    }
}

impl Drop for Ext2FileSystem {
    fn drop(&mut self) {
        self.write_meta();
        block_cache_sync_all();
    }
}