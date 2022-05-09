use core::mem;

use log::info;

use crate::device::BlockDevice;
use crate::error::Result;
use crate::lock::MutexTrait;
use crate::utils::get_params;
use crate::{ArcMutex, RawFatEntry, SectorId};
use crate::{CachedPartition, ClusterId, FatEntry};

fn read_fat_entry(
    cluster_id: ClusterId,
    sector_size: usize,
    device: ArcMutex<CachedPartition>,
    fat_start_sector: SectorId,
) -> Result<FatEntry> {
    let (sector, offset) = get_params(sector_size, fat_start_sector, cluster_id)?;

    // so the this cluster id, is the 6th element of the 12th sector.
    info!(
        "Requested cid: {}, sector: {}, offset in sector: {}",
        cluster_id, sector, offset
    );
    let mut buf = [0u8; mem::size_of::<RawFatEntry>()];
    let mut mutex = device.as_ref();
    mutex
        .lock(|dev| dev.read_sector_offset(sector, offset, &mut buf))
        .map(|_| {
            let raw_entry = RawFatEntry::new(buf);
            FatEntry::from(raw_entry)
        })
}

/// Returns the next clusterid in the chain after the provided cluster_id, if any.
pub fn next_cluster(
    cluster_id: ClusterId,
    sector_size: usize,
    device: ArcMutex<CachedPartition>,
    fat_start_sector: SectorId,
) -> Result<Option<ClusterId>> {
    let fat_entry = read_fat_entry(cluster_id, sector_size, device, fat_start_sector)?;
    info!("Fat entry: {:?}", fat_entry);
    Ok(match fat_entry {
        FatEntry::DataCluster(id) => Some(ClusterId::new(id)),
        _ => None,
    })
}
