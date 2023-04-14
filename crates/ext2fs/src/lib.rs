#![no_std]
extern crate alloc;
mod layout;
mod config;
mod block_cache;
mod block_dev;
mod bitmap;
mod efs;
mod vfs;

pub use block_dev::BlockDevice;
pub use efs::Ext2FileSystem;
pub use vfs::Inode;
pub use config::{BLOCK_SIZE, BLOCKS_PER_GRP};
pub use layout::{EXT2_S_IFREG, EXT2_S_IFDIR};
use block_cache::{block_cache_sync_all, get_block_cache};
use bitmap::Bitmap;
use layout::{SuperBlock, DiskInode, BlockGroupDesc};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
