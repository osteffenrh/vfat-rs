use crate::device::BlockDevice;
use crate::error::Result;
use crate::fat_table::fat_entry::FAT_ENTRY_SIZE;
use crate::VfatRsError::CheckedMulFailed;
use crate::{error, ClusterId, FatEntry, SectorId};
use alloc::boxed::Box;
use log::info;

pub(crate) struct CachedPartition {
    device: Box<dyn BlockDevice>,
    pub(crate) sector_size: usize,
    pub(crate) fat_start_sector: SectorId,
}
impl CachedPartition {
    pub fn new<T>(device: T, sector_size: usize, fat_start_sector: SectorId) -> Self
    where
        T: BlockDevice + 'static,
    {
        Self {
            device: Box::new(device),
            sector_size,
            fat_start_sector,
        }
    }
    /// Given a cluster_id, returns the sector id to read in order to get the FAT table entry for
    /// this cluster id.
    pub(crate) fn get_params(&self, cluster_id: ClusterId) -> error::Result<(SectorId, usize)> {
        // this should be 512 / 32 = 18
        let fat_entries_per_sector = self.sector_size / FAT_ENTRY_SIZE;
        // In which sector is this cid contained. Cid: 222 / 18 = 12.3333
        //TODO: check floor
        let containing_sector = (f64::from(cluster_id) / fat_entries_per_sector as f64) as u32;
        // The sector is 12, now let's calculate the offset in that sector: 222 % 18 = 6.
        // TODO: check floor
        let offset_in_sector = ((f64::from(cluster_id) % fat_entries_per_sector as f64) as usize)
            .checked_mul(FAT_ENTRY_SIZE)
            .ok_or(CheckedMulFailed)?; //Todo: make nicer.

        let sector = SectorId(self.fat_start_sector + containing_sector);

        Ok((sector, offset_in_sector))
    }

    pub(crate) fn read_fat_entry(&mut self, cluster_id: ClusterId) -> Result<FatEntry> {
        let mut buf = [0u8; FAT_ENTRY_SIZE];
        let (sector, offset) = self.get_params(cluster_id)?;
        info!(
            "Requested cid: {}, sector: {}, offset in sector: {}",
            cluster_id, sector, offset
        );
        self.read_sector_offset(sector, offset, &mut buf)
            .map(|_| FatEntry::from(buf))
    }

    pub(crate) fn set_fat_entry(&mut self, cluster_id: ClusterId, entry: FatEntry) -> Result<()> {
        let (sector, offset) = self.get_params(cluster_id)?;

        info!(
            "Requested cid: {}, containing sector: {}, offset in sector: {}",
            cluster_id, sector, offset
        );
        self.write_sector_offset(sector, offset, &entry.as_buff())?;
        Ok(())
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
