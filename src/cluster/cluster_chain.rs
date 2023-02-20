use crate::cache::CachedPartition;
use crate::{fat_table, ArcMutex, ClusterId, Result, SectorId, VfatFS};
use log::{debug, info};

/// A reader for the content of a single FAT cluster
/// It reads from the beginning of the cluster.
/// It's not thread-safe and should not be shared.
/// Given a buffer buf, it will try to read as much sectors as it can in order to fill the buffer.
#[derive(Clone)]
pub struct ClusterChain {
    vfat_fs: VfatFS,
    current_cluster: ClusterId,
}
impl ClusterChain {
    /*
    pub(crate) fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.current_cluster.is_none() || buf.is_empty() {
            return Ok(0);
        }

        let mut amount_read = 0;
        while amount_read < buf.len() && self.current_cluster.is_some() {
            debug!("CCR: amount_read: {}", amount_read);
            // TODO: to allow tracking of last written cluster from external user of this struct
            self.last_cluster_read = self.current_cluster.unwrap();
            let current_amount_read = self.cluster_reader.read(&mut buf[amount_read..])?;
            debug!("CCR: current_amount_read: {}", current_amount_read);
            amount_read += current_amount_read;
            if current_amount_read == 0 {
                self.current_cluster = self.next_cluster()?;
                if self.current_cluster.is_some() {
                    info!("Using next cluster: {:?}", self.current_cluster);
                    // If there is another cluster in the chain,
                    // create a new cluster reader.
                    self.cluster_reader = self.cluster_reader_builder();
                }
            }
        }
        debug!(
            "CRR completed, red<buf: {}, is some: {}",
            amount_read < buf.len(),
            self.current_cluster.is_some()
        );

        Ok(amount_read)
    }

     */
}
