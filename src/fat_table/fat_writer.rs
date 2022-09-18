use crate::error::Result;
use crate::{ArcMutex, CachedPartition, ClusterId, FatEntry};

/// Delete a cluster chain starting from `current`.
/// TODO: Start from the end of the chain to make the operation safer.
/// TODO: Check if "current" is of "Used" type.
/// TODO: Test with array backed dev.
pub(crate) fn delete_cluster_chain(
    mut current: ClusterId,
    device: ArcMutex<CachedPartition>,
) -> Result<()> {
    const DELETED_ENTRY: FatEntry = FatEntry::Unused;
    while let Some(next) = crate::fat_reader::next_cluster(current, device.clone())? {
        let mut dev_lock = device.lock();
        dev_lock.set_fat_entry(current, DELETED_ENTRY)?;
        current = next;
    }

    let mut dev_lock = device.lock();
    dev_lock.set_fat_entry(current, DELETED_ENTRY)?;

    Ok(())
}
