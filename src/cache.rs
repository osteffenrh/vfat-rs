use crate::device::BlockDevice;
use crate::error::Result;
use crate::SectorId;
use alloc::boxed::Box;

pub struct CachedPartition {
    device: Box<dyn BlockDevice>,
}
impl CachedPartition {
    pub fn new<T>(device: T) -> Self
    where
        T: BlockDevice + 'static,
    {
        Self {
            device: Box::new(device),
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        todo!()
    }
}
/*
TODO:
impl Drop for CachedPartition {
    fn drop(&mut self) {
        unimplemented!()
    }
}
 */
impl BlockDevice for CachedPartition {
    fn read_sector(&mut self, sector: SectorId, buf: &mut [u8]) -> Result<usize> {
        self.device.read_sector(sector, buf)
    }
    fn read_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize> {
        self.device.read_sector_offset(sector, offset, buf)
    }

    fn write_sector(&mut self, sector: SectorId, buf: &[u8]) -> Result<usize> {
        self.device.write_sector(sector, buf)
    }

    fn write_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        buf: &[u8],
    ) -> Result<usize> {
        self.device.write_sector_offset(sector, offset, buf)
    }

    fn get_canonical_name() -> &'static str
    where
        Self: Sized,
    {
        "CachePartition"
    }
}
