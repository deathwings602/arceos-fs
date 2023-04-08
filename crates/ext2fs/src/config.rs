#![allow(unused)]
pub const BLOCK_SIZE: usize = 4096;
pub const LOG_BLOCK_SIZE: usize = 2;
pub const LOG_FRAG_SIZE: usize = LOG_BLOCK_SIZE;
pub const INODES_PER_GRP: usize = 8 * BLOCK_SIZE;
pub const BLOCKS_PER_GRP: usize = 8 * BLOCK_SIZE;
pub const FIRST_DATA_BLOCK: usize = if BLOCK_SIZE > 1024 { 0 } else { 1 };
pub const FAKE_CREATE_TIME: usize = 50 * 365 * 24 * 3600;
pub const CHECK_INTERVAL: usize = 3 * 30 * 24 * 3600;
pub const EXT2_GOOD_OLD_FIRST_INO: usize = 11;
pub const EXT2_GOOD_OLD_INODE_SIZE: usize = 128;
pub const FAKE_UUID: u128 = 114514;
pub const FAKE_JOURNAL_UUID: u128 = 996;