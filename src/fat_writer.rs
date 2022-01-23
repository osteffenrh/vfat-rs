use log::info;

use crate::device::BlockDevice;
use crate::error::Result;
use crate::utils::get_params;
use crate::{ArcMutex, CachedPartition, ClusterId, FatEntry, MutexTrait, SectorId};

pub fn set_fat_entry(
    cluster_id: ClusterId,
    sector_size: usize,
    device: ArcMutex<CachedPartition>,
    fat_start_sector: SectorId,
    entry: FatEntry,
) -> Result<()> {
    let (sector, offset) = get_params(sector_size, fat_start_sector, cluster_id)?;

    info!(
        "Requested cid: {}, containing sector: {}, offset in sector: {}",
        cluster_id, sector, offset
    );
    let mut mutex = device.as_ref();
    mutex.lock(|dev| dev.write_sector_offset(sector, offset, &entry.as_buff()))?;
    Ok(())
}

/// Delete a cluster chain starting from `current`.
/// TODO: Start from the end of the chain to make the operation safer.
/// TODO: Check if "current" is of "Used" type.
pub fn delete_cluster_chain(
    mut current: ClusterId,
    sector_size: usize,
    device: ArcMutex<CachedPartition>,
    fat_start_sector: SectorId,
) -> Result<()> {
    const DELETED_ENTRY: FatEntry = FatEntry::Unused;
    while let Some(next) =
        crate::fat_reader::next_cluster(current, sector_size, device.clone(), fat_start_sector)?
    {
        set_fat_entry(
            current,
            sector_size,
            device.clone(),
            fat_start_sector,
            DELETED_ENTRY,
        )?;
        current = next;
    }
    Ok(())
}
