use log::info;

use crate::error::Result;
use crate::fat_table::{get_params, FatEntry};
use crate::{fat_table, ArcMutex, BlockDevice, CachedPartition, ClusterId};

/// Delete a cluster chain starting from `current`.
/// TODO: Start from the end of the chain to make the operation safer.
/// TODO: Check if "current" is of "Used" type.
/// TODO: Test with array backed dev.
pub(crate) fn delete_cluster_chain(
    mut current: ClusterId,
    device: ArcMutex<CachedPartition>,
) -> Result<()> {
    const DELETED_ENTRY: FatEntry = FatEntry::Unused;
    while let Some(next) = fat_table::next_cluster(current, device.clone())? {
        let mut dev_lock = device.lock();
        set_fat_entry(&mut dev_lock, current, DELETED_ENTRY)?;
        current = next;
    }

    let mut dev_lock = device.lock();
    set_fat_entry(&mut dev_lock, current, DELETED_ENTRY)?;

    Ok(())
}

pub(crate) fn set_fat_entry(
    device: &mut CachedPartition,
    cluster_id: ClusterId,
    entry: FatEntry,
) -> Result<()> {
    let (sector, offset) = get_params(device, cluster_id)?;

    info!(
        "Requested cid: {}, containing sector: {}, offset in sector: {}",
        cluster_id, sector, offset
    );
    device.write_sector_offset(sector, offset, &entry.as_buff())?;
    Ok(())
}
