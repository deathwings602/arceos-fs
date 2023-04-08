#![allow(unused)]
use super::config::*;
use _core::mem::size_of;
use bitflags::*;
use alloc::string::String;


const VOLUMN_NAME_SIZE: usize = 16;
const MOUNT_SIZE: usize = 64;
const HASH_SEED_SIZE: usize = 4;
const SB_RESERVED_SIZE: usize = 760;

pub const DIRECT_BLOCK_NUM: usize = 13;
pub const DOUBLE_BLOCK_NUM: usize = BLOCK_SIZE/4;
pub const TRIPLE_BLOCK_NUM: usize = (BLOCK_SIZE/4) * (BLOCK_SIZE/4);
pub const SB_MAGIC: u16 = 0xEF53;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SuperBlock {
    s_inodes_count: u32,
    s_blocks_count: u32,
    s_r_blocks_count: u32,
    s_free_blocks_count: u32,
    s_free_inodes_count: u32,
    s_first_data_block: u32,
    s_log_block_size: u32,
    s_log_frag_size: u32,
    s_blocks_per_group: u32,
    s_frags_per_group: u32,
    s_inodes_per_group: u32,
    s_mtime: u32,
    s_wtime: u32,
    s_mnt_count: u16,
    s_max_mnt_count: u16,
    s_magic: u16,
    s_state: u16,
    s_errors: u16,
    s_minor_rev_level: u16,
    s_lastcheck: u32,
    s_checkinterval: u32,
    s_creator_os: u32,
    s_rev_level: u32,
    s_def_resuid: u16,
    s_def_resgid: u16,
    // EXT2_DYNAMIC_REV Specific
    s_first_ino: u32,
    s_inode_size: u16,
    s_block_group_nr: u16,
    s_feature_compat: FeatureCompat,
    s_feature_incompat: FeatureIncompat,
    s_feature_ro_compat: FeatureRocompat,
    s_uuid: u128,
    s_volume_name: [u8; VOLUMN_NAME_SIZE],
    s_last_mounted: [u8; MOUNT_SIZE],
    s_algo_bitmap: u32,
    // Performance hints
    s_prealloc_blocks: u8,
    s_prealloc_dir_blocks: u8,
    p_padding: [u8; 2],
    // Journaling Support
    s_journal_uuid: u128,
    s_journal_inum: u32,
    s_journal_dev: u32,
    s_last_orphan: u32,
    // Directory Indexing Support
    s_hash_seed: [u32; HASH_SEED_SIZE],
    s_def_hash_version: u8,
    i_padding: [u8; 3],
    // Other options
    s_default_mount_option: u32,
    s_first_meta_bg: u32,
    reserved: [u8; SB_RESERVED_SIZE]
}

// s_state
const EXT2_VALID_FS: u16 = 1;
const EXT2_ERROR_FS: u16 = 2;

// s_errors
const EXT2_ERRORS_CONTINUE: u16 = 1;
const EXT2_ERRORS_RO: u16 = 2;
const EXT2_ERRORS_PANIC: u16 = 3;

// s_creator_os
const EXT2_OS_LINUX: u32 = 0;
const EXT2_OS_HURD: u32 = 1;
const EXT2_OS_MASIX: u32 = 2;
const EXT2_OS_FREEBSD: u32 = 3;
const EXT2_OS_LITES: u32 = 4;

// s_rev_level
const EXT2_GOOD_OLD_REV: u32 = 0;
const EXT2_DYNAMIC_REV: u32 = 1;

// s_def_resuid
const EXT2_DEF_RESUID: u16 = 0;

// s_def_resgid
const EXT2_DEF_RESGID: u16 = 0;

// s_feature_compat

bitflags! {
    pub struct FeatureCompat: u32 {
        const EXT2_FEATURE_COMPAT_DIR_PREALLOC = 1;
        const EXT2_FEATURE_COMPAT_IMAGIC_INODES = 1 << 1;
        const EXT3_FEATURE_COMPAT_HAS_JOURNAL = 1 << 2;
        const EXT2_FEATURE_COMPAT_EXT_ATTR = 1 << 3;
        const EXT2_FEATURE_COMPAT_RESIZE_INO = 1 << 4;
        const EXT2_FEATURE_COMPAT_DIR_INDEX = 1 << 5;
    }
}

bitflags! {
    pub struct FeatureIncompat: u32 {
        const EXT2_FEATURE_INCOMPAT_COMPRESSION = 1;
        const EXT2_FEATURE_INCOMPAT_FILETYPE = 1 << 1;
        const EXT3_FEATURE_INCOMPAT_RECOVER = 1 << 2;
        const EXT3_FEATURE_INCOMPAT_JOURNAL_DEV = 1 << 3;
        const EXT2_FEATURE_INCOMPAT_META_BG = 1 << 4;
    }
}

bitflags! {
    pub struct FeatureRocompat: u32 {
        const EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER = 1;
        const EXT2_FEATURE_RO_COMPAT_LARGE_FILE = 1 << 1;
        const EXT2_FEATURE_RO_COMPAT_BTREE_DIR = 1 << 2;
    }
}

bitflags! {
    pub struct AlgoBitmap: u32 {
        const EXT2_LZV1_ALG = 1;
        const EXT2_LZRW3A_ALG = 1 << 1;
        const EXT2_GZIP_ALG = 1 << 2;
        const EXT2_BZIP2_ALG = 1 << 3;
        const EXT2_LZO_ALG = 1 << 4;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct BlockGroupDesc {
    bg_block_bitmap: u32,
    bg_inode_bitmap: u32,
    bg_inode_table: u32,
    bg_free_blocks_count: u16,
    bg_free_inodes_count: u16,
    bg_used_dirs_count: u16,
    bg_pad: u16,
    bg_reserved: [u8; 12]
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct DiskInode {
    i_mode: u16,
    i_uid: u16,
    i_size: u32,
    i_atime: u32,
    i_ctime: u32,
    i_dtime: u32,
    i_gid: u16,
    i_links_count: u16,
    // the total number of 512-bytes blocks
    i_blocks: u32,
    i_flags: u32,
    i_osd1: u32,
    i_direct_block: [u32; DIRECT_BLOCK_NUM],
    i_double_block: u32,
    i_triple_block: u32,
    i_generation: u32,
    i_file_acl: u32,
    i_dir_acl: u32,
    i_faddr: u32,
    i_osd2: LinuxOSD
}

// Defined Reserved Inodes
const EXT2_BAD_INO: u32 = 1;
const EXT2_ROOT_INO: u32 = 2;
const EXT2_ACL_IDX_INO: u32 = 3;
const EXT2_ACL_DATA_INO: u32 = 4;
const EXT2_BOOT_LOADER_INO: u32 = 5;
const EXT2_UNDEL_DIR_INO: u32 = 6;

bitflags! {
    pub struct IMODE: u16 {
        // access control
        const EXT2_S_IXOTH = 1;
        const EXT2_S_IWOTH = 1 << 1;
        const EXT2_S_IROTH = 1 << 2;
        const EXT2_S_IXGRP = 1 << 3;
        const EXT2_S_IWGRP = 1 << 4;
        const EXT2_S_IRGRP = 1 << 5;
        const EXT2_S_IXUSR = 1 << 6;
        const EXT2_S_IWUSR = 1 << 7;
        const EXT2_S_IRUSR = 1 << 8;
        // process
        const EXT2_S_ISVTX = 1 << 9;
        const EXT2_S_ISGID = 1 << 10;
        const EXT2_S_ISUID = 1 << 11;
    }
}

// IMODE -> file format
const EXT2_S_IFIFO: u16 = 0x1000;
const EXT2_S_IFCHR: u16 = 0x2000;
const EXT2_S_IFDIR: u16 = 0x4000;
const EXT2_S_IFBLK: u16 = 0x6000;
const EXT2_S_IFREG: u16 = 0x8000;
const EXT2_S_IFLNK: u16 = 0xA000;
const EXT2_S_IFSOCK: u16 = 0xC000;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct LinuxOSD {
    l_i_frag: u8,
    l_i_fsize: u8,
    reserved: [u8; 2],
    l_i_uid_high: u16,
    l_i_gid_high: u16,
    reserved_2: [u8; 4]
}

impl LinuxOSD {
    fn empty() -> LinuxOSD {
        LinuxOSD {
            l_i_frag: 0,
            l_i_fsize: 0,
            reserved: [0; 2],
            l_i_uid_high: 0,
            l_i_gid_high: 0,
            reserved_2: [0; 4]
        }
    }
}


#[derive(Clone, Copy)]
#[repr(C)]
pub struct DirEntryHead {
    inode: u32,
    rec_len: u16,
    name_len: u8,
    file_type: u8,
    // name is variable length
}

impl SuperBlock {
    fn new(
        inodes_count: usize,
        blocks_count: usize,
        free_inodes_count: usize,
        free_blocks_count: usize,
        block_group_num: usize,
        volumn_name: &str
    ) -> SuperBlock 
    {
        let mut sb = SuperBlock {
            s_inodes_count: inodes_count as u32,
            s_blocks_count: blocks_count as u32,
            s_r_blocks_count: 0,
            s_free_blocks_count: free_blocks_count as u32,
            s_free_inodes_count: free_inodes_count as u32,
            s_first_data_block: FIRST_DATA_BLOCK as u32,
            s_log_block_size: LOG_BLOCK_SIZE as u32,
            s_log_frag_size: LOG_FRAG_SIZE as u32,
            s_blocks_per_group: BLOCKS_PER_GRP as u32,
            s_frags_per_group: BLOCKS_PER_GRP as u32,
            s_inodes_per_group: INODES_PER_GRP as u32,
            s_mtime: FAKE_CREATE_TIME as u32,
            s_wtime: FAKE_CREATE_TIME as u32,
            s_mnt_count: 0,
            s_max_mnt_count: 32,
            s_magic: SB_MAGIC,
            s_state: EXT2_VALID_FS,
            s_errors: EXT2_ERRORS_RO,
            s_minor_rev_level: 0,
            s_lastcheck: FAKE_CREATE_TIME as u32,
            s_checkinterval: CHECK_INTERVAL as u32,
            s_creator_os: EXT2_OS_LINUX,
            s_rev_level: EXT2_GOOD_OLD_REV,
            s_def_resuid: EXT2_DEF_RESUID,
            s_def_resgid: EXT2_DEF_RESGID,
            s_first_ino: EXT2_GOOD_OLD_FIRST_INO as u32,
            s_inode_size: EXT2_GOOD_OLD_INODE_SIZE as u16,
            s_block_group_nr: block_group_num as u16,
            s_feature_compat: FeatureCompat::from_bits_truncate(0),
            s_feature_incompat: FeatureIncompat::from_bits_truncate(0),
            s_feature_ro_compat: FeatureRocompat::from_bits_truncate(0),
            s_uuid: FAKE_UUID,
            s_volume_name: [0; VOLUMN_NAME_SIZE],
            s_algo_bitmap: 0, // we don't use compression
            s_prealloc_blocks: 0,
            s_last_mounted: [0; MOUNT_SIZE],
            i_padding: [0; 3],
            s_prealloc_dir_blocks: 0,
            p_padding: [0; 2],
            s_journal_uuid: FAKE_JOURNAL_UUID,
            s_journal_inum: 0,
            s_journal_dev: 0,
            s_last_orphan: 0,
            s_hash_seed: [0; HASH_SEED_SIZE],
            s_def_hash_version: 0,
            s_default_mount_option: 0,
            s_first_meta_bg: 0,
            reserved: [0; SB_RESERVED_SIZE]
        };
        sb.s_volume_name[..volumn_name.len()].copy_from_slice(volumn_name.as_bytes());
        sb
    }

    fn is_valid(&self) -> bool {
        self.s_magic == SB_MAGIC
    }
}

impl BlockGroupDesc {
    fn new(
        block_bitmap: usize,
        inode_bitmap: usize,
        inode_table: usize,
        free_blocks: usize,
        free_inodes: usize,
        used_dirs: usize,
    ) -> BlockGroupDesc
    {
        BlockGroupDesc {
            bg_block_bitmap: block_bitmap as u32,
            bg_inode_bitmap: inode_bitmap as u32,
            bg_inode_table: inode_table as u32,
            bg_free_blocks_count: free_blocks as u16,
            bg_free_inodes_count: free_inodes as u16,
            bg_used_dirs_count: used_dirs as u16,
            bg_pad: 0,
            bg_reserved: [0; 12]
        }
    }
}

impl DiskInode {
    fn new(
        acl_mode: IMODE,
        file_type: u16,
        uid: usize,
        gid: usize,

    ) -> DiskInode
    {
        DiskInode {
            i_mode: acl_mode.bits() | file_type,
            i_uid: uid as u16,
            i_size: 0,
            i_atime: 0,
            i_ctime: 0,
            i_dtime: 0,
            i_gid: gid as u16,
            i_links_count: 1,
            i_blocks: 0,
            i_flags: 0,
            i_osd1: 0,
            i_direct_block: [0; DIRECT_BLOCK_NUM],
            i_double_block: 0,
            i_triple_block: 0,
            i_generation: 0,
            i_dir_acl: 0,
            i_file_acl: 0,
            i_faddr: 0,
            i_osd2: LinuxOSD::empty()
        }
    }
}


impl DirEntryHead {
    fn create(inode: usize, name: &str, file_type: u8) -> DirEntryHead {
        let name_len = name.len();
        let rec_len = size_of::<DirEntryHead>() + name_len;

        DirEntryHead {
            inode: inode as u32,
            rec_len: rec_len as u16,
            name_len: name_len as u8,
            file_type: file_type
        }
    }
}
