cfg_if::cfg_if! {
    if #[cfg(feature = "fatfs")] {
        pub mod fatfs;
    } else if #[cfg(feature = "ext2fs")] {
        pub mod ext2fs;
    }
}

#[cfg(feature = "devfs")]
pub use axfs_devfs as devfs;
