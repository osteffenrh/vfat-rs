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

    pub fn seek(&mut self, offset: usize) -> Result<()> {
        // Calculate in which cluster this offset falls:
        let cluster_size =
            self.vfat_fs.device.sectors_per_cluster as usize * self.vfat_fs.device.sector_size;
        let cluster_offset = (offset as f64 / cluster_size as f64) as usize; //TODO: check it's floor()

        // Calculate in which sector this offset falls:
        let sector_offset = offset / self.vfat_fs.device.sector_size
            % self.vfat_fs.device.sectors_per_cluster as usize;

        // Finally, calculate the offset in the selected sector:
        let offset_in_sector = offset % self.vfat_fs.device.sector_size;
        info!(
            "Offset: {}, cluster_offset: {}, sector offset: {}, offset in sector: {}, current cluster: {:?}",
            offset, cluster_offset, sector_offset, offset_in_sector, self.current_cluster
        );
        for _ in 0..cluster_offset {
            self.current_cluster = self.next_cluster_alloc()?;
        }
        info!("Current cluster: {:?}", self.current_cluster);
        self.cluster_writer = ClusterWriter::new_offset(
            self.vfat_fs.device.clone(),
            self.vfat_fs.device.cluster_to_sector(self.current_cluster),
            SectorId(sector_offset as u32),
            offset_in_sector,
        );

        Ok(())
    }

    fn next_cluster(&self) -> Result<Option<ClusterId>> {
        if self.current_cluster.is_none() {
            return Ok(None);
        }
        fat_table::next_cluster(self.current_cluster.unwrap(), self.device.clone())
    }
    /// Allocates cluster if needed
    fn next_cluster_alloc(&self) -> Result<ClusterId> {
        let ret = self.next_cluster();
        Ok(match ret {
            None => self
                .vfat_fs
                .allocate_cluster_to_chain(self.current_cluster)?,
            Some(r) => r,
        })
    }
    */
}
