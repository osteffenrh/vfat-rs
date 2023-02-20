use alloc::boxed::Box;
use alloc::sync::Arc;

use log::info;
use spin::mutex::SpinMutex;

use crate::device::BlockDevice;
use crate::error::Result;
use crate::SectorId;

pub(crate) struct CachedPartition {
    device: SpinMutex<Box<dyn BlockDevice>>,
    pub(crate) sector_size: usize,
    pub(crate) fat_start_sector: SectorId,
}
impl CachedPartition {
    pub fn new<T>(device: T, sector_size: usize, fat_start_sector: SectorId) -> Self
    where
        T: BlockDevice + 'static,
    {
        info!("Creating cached partition");
        Self {
            device: SpinMutex::new(Box::new(device)),
            sector_size,
            fat_start_sector,
        }
    }

    pub fn flush(&self) -> Result<()> {
        todo!()
    }

    pub(crate) fn read_sector(self: Arc<Self>, sector: SectorId, buf: &mut [u8]) -> Result<usize> {
        let mut dev_lock = self.device.lock();
        dev_lock.read_sector(sector, buf)
    }

    pub(crate) fn read_sector_offset(
        self: Arc<Self>,
        sector: SectorId,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize> {
        let mut dev_lock = self.device.lock();
        dev_lock.read_sector_offset(sector, offset, buf)
    }
    #[allow(unused)]
    fn write_sector(self: Arc<Self>, sector: SectorId, buf: &[u8]) -> Result<usize> {
        let mut dev_lock = self.device.lock();
        dev_lock.write_sector(sector, buf)
    }

    pub(crate) fn write_sector_offset(
        self: Arc<Self>,
        sector: SectorId,
        offset: usize,
        buf: &[u8],
    ) -> Result<usize> {
        let mut dev_lock = self.device.lock();
        dev_lock.write_sector_offset(sector, offset, buf)
    }

    #[allow(unused)]
    fn get_canonical_name() -> &'static str
    where
        Self: Sized,
    {
        "CachePartition"
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
