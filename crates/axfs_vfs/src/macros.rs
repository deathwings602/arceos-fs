#[macro_export]
macro_rules! impl_vfs_dir_default {
    () => {
        fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> $crate::VfsResult<usize> {
            $crate::__priv::ax_err!(IsADirectory)
        }

        fn write_at(&self, _offset: u64, _buf: &[u8]) -> $crate::VfsResult<usize> {
            $crate::__priv::ax_err!(IsADirectory)
        }

        fn fsync(&self) -> $crate::VfsResult {
            $crate::__priv::ax_err!(IsADirectory)
        }

        fn truncate(&self, _size: u64) -> $crate::VfsResult {
            $crate::__priv::ax_err!(IsADirectory)
        }
    };
}

#[macro_export]
macro_rules! impl_vfs_non_dir_default {
    () => {
        fn lookup(
            self: $crate::__priv::Arc<Self>,
            _path: &str,
        ) -> $crate::VfsResult<$crate::VfsNodeRef> {
            $crate::__priv::ax_err!(NotADirectory)
        }

        fn create(&self, _path: &str, _ty: $crate::VfsNodeType) -> $crate::VfsResult {
            $crate::__priv::ax_err!(NotADirectory)
        }

        fn remove(&self, _path: &str, _recursive: bool) -> $crate::VfsResult {
            $crate::__priv::ax_err!(NotADirectory)
        }

        fn read_dir(
            &self,
            _start_idx: usize,
            _dirents: &mut [$crate::VfsDirEntry],
        ) -> $crate::VfsResult<usize> {
            $crate::__priv::ax_err!(NotADirectory)
        }

        fn symlink(&self, _name: &str, _path: &str) -> $crate::VfsResult {
            $crate::__priv::ax_err!(NotADirectory)
        }

        fn link(&self, _name: &str, _handle: &$crate::LinkHandle) -> $crate::VfsResult {
            $crate::__priv::ax_err!(NotADirectory)
        }
    };
}

#[macro_export]
macro_rules! impl_ext2_common {
    () => {
        fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
            self.0
                .disk_inode()
                .map(|disk_inode| {
                    let (ty, perm) = map_imode(disk_inode.i_mode);
                    VfsNodeAttr::new(perm, ty, disk_inode.i_size as _, disk_inode.i_blocks as _)
                })
                .map_err(map_ext2_err)
        }
    };
}

#[macro_export]
macro_rules! impl_ext2_linkable {
    () => {
        fn get_link_handle(&self) -> VfsResult<$crate::LinkHandle> {
            let inode_id = self.0.inode_id().map_err(map_ext2_err)?;
            Ok($crate::LinkHandle {
                inode_id,
                fssp_ptr: self.1.as_ptr() as usize,
            })
        }
    };
}
