use jbd_rs::sal::{Buffer, BufferProvider, System};
use crate::block_cache_manager::BlockCacheWrapper;
use crate::Ext2FileSystem;
use alloc::boxed::Box;
use core::any::Any;
use alloc::sync::Arc;
use core::cell::RefCell;

impl Buffer for BlockCacheWrapper {
    fn block_id(&self) -> usize {
        unsafe {self.unsafe_get_mut().block_id}
    }
    fn size(&self) -> usize {
        crate::BLOCK_SIZE
    }
    fn dirty(&self) -> bool {
        unsafe { self.unsafe_get_mut().modified }
    }
    fn data(&self) -> *mut u8 {
        unsafe { self.unsafe_get_mut().cache.as_mut_ptr() }
    }
    fn private(&self) -> &Option<Box<dyn Any>> {
        unsafe { &self.unsafe_get_mut().private }
    }
    fn set_private(&self, private: Option<Box<dyn Any>>) {
        unsafe { self.unsafe_get_mut().private = private }
    }
    fn mark_dirty(&self) {
        unsafe { self.unsafe_get_mut().modified = true }
    }
    fn clear_dirty(&self) {
        unsafe { self.unsafe_get_mut().modified = false }
    }
    fn test_clear_dirty(&self) -> bool {
        unsafe {
            let origin = self.unsafe_get_mut().modified;
            self.unsafe_get_mut().modified = false;
            origin
        }
    }
    fn jbd_managed(&self) -> bool {
        unsafe { self.unsafe_get_mut().jbd_managed_ }
    }
    fn set_jbd_managed(&self, managed: bool) {
        unsafe { self.unsafe_get_mut().jbd_managed_ = managed }
    }
    fn mark_jbd_dirty(&self) {
        unsafe { self.unsafe_get_mut().jbd_dirty_ = true }
    }
    fn clear_jbd_dirty(&self) {
        unsafe { self.unsafe_get_mut().jbd_dirty_ = false }
    }
    fn test_clear_jbd_dirty(&self) -> bool {
        unsafe {
            let origin = self.unsafe_get_mut().jbd_dirty_;
            self.unsafe_get_mut().jbd_dirty_ = false;
            origin
        }
    }
    fn jbd_dirty(&self) -> bool {
        unsafe { self.unsafe_get_mut().jbd_dirty_ }
    }
    fn revoked(&self) -> bool {
        unsafe { self.unsafe_get_mut().revoked_ }
    }
    fn set_revoked(&self) {
        unsafe { self.unsafe_get_mut().revoked_ = true }
    }
    fn test_clear_revoked(&self) -> bool {
        unsafe {
            let origin = self.unsafe_get_mut().revoked_;
            self.unsafe_get_mut().revoked_ = false;
            origin
        }
    }
    fn clear_revoked(&self) {
        unsafe { self.unsafe_get_mut().revoked_ = false }
    }
    fn test_set_revoked(&self) -> bool {
        unsafe {
            let origin = self.unsafe_get_mut().revoked_;
            self.unsafe_get_mut().revoked_ = true;
            origin
        }
    }
    fn revoke_valid(&self) -> bool {
        unsafe { self.unsafe_get_mut().revoke_valid_ }
    }
    fn set_revoke_valid(&self) {
        unsafe { self.unsafe_get_mut().revoke_valid_ = true }
    }
    fn test_set_revoke_valid(&self) -> bool {
        unsafe {
            let origin = self.unsafe_get_mut().revoke_valid_;
            self.unsafe_get_mut().revoke_valid_ = true;
            origin
        }
    }
    fn clear_revoke_valid(&self) {
        unsafe { self.unsafe_get_mut().revoke_valid_ = false }
    }
    fn test_clear_revoke_valid(&self) -> bool {
        unsafe {
            let origin = self.unsafe_get_mut().revoke_valid_;
            self.unsafe_get_mut().revoke_valid_ = false;
            origin
        }
    }

}

impl BufferProvider for Ext2FileSystem {
    fn get_buffer(&self, _dev: &alloc::sync::Arc<dyn jbd_rs::sal::BlockDevice>, block_id: usize) -> Option<Arc<dyn Buffer>> {
        Some(self.manager.lock().get_block_cache(block_id))
    }
    fn sync(&self, _dev: &Arc<dyn jbd_rs::sal::BlockDevice>, buf: Arc<dyn Buffer>) {
        unsafe { self.manager.lock().unsafe_write_by_id(buf.block_id()) }
    }
}

impl System for Ext2FileSystem {
    fn get_buffer_provider(&self) -> Arc<dyn BufferProvider> {
        self.inner.lock().to_self.as_ref().unwrap().upgrade().unwrap()
    }
    fn get_time(&self) -> usize {
        self.timer.get_current_time() as _
    }
    fn set_current_handle(&self, handle: Option<Arc<RefCell<jbd_rs::Handle>>>) {
        self.inner.lock().handle_ = handle;
    }
    fn get_current_handle(&self) -> Option<Arc<RefCell<jbd_rs::Handle>>> {
        self.inner.lock().handle_.clone()
    }
}