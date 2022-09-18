use log::info;

use crate::error::Result;
use crate::ArcMutex;
use crate::{CachedPartition, ClusterId, FatEntry};

/// Returns the next clusterid in the chain after the provided cluster_id, if any.
/// To do that, query the fat table for this cluster id, and see if it is a Data Cluster (e.g. a
/// node in the chain) return the next element, otherwise it's a dead end and return null.
pub(crate) fn next_cluster(
    cluster_id: ClusterId,
    device: ArcMutex<CachedPartition>,
) -> Result<Option<ClusterId>> {
    let mut dev_lock = device.as_ref().lock();
    let fat_entry = dev_lock.read_fat_entry(cluster_id)?;
    info!("Fat entry: {:?}", fat_entry);
    Ok(match fat_entry {
        FatEntry::DataCluster(id) => Some(ClusterId::new(id)),
        _ => None,
    })
}
