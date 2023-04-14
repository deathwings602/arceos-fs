use core::any::Any;
/// Trait for block devices
/// which reads and writes data in the unit of blocks
pub trait BlockDevice: Send + Sync + Any {
    /// Read data form block to buffer
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    /// Write data from buffer to block
    fn write_block(&self, block_id: usize, buf: &[u8]);
    /// Get block size
    fn block_size(&self) -> usize;
    /// Get block num
    fn block_num(&self) -> usize;
}