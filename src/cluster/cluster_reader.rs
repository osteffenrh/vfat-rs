use log::{debug, info};

use crate::cache::CachedPartition;
use crate::{fat_table, ArcMutex, ClusterId, Result, SectorId};

/// A reader for the content of a single FAT cluster
/// It reads from the beginning of the cluster.
/// It's not thread-safe and should not be shared.
/// Given a buffer buf, it will try to read as much sectors as it can in order to fill the buffer.
struct ClusterReader {
    pub device: ArcMutex<CachedPartition>,
    pub current_sector: SectorId,
    /// Offset in current_sector. In case buf.len()%sector_size != 0, this sector is not full read.
    /// The next read call will start from this offset.
    pub offset_byte_in_current_sector: usize,
    final_sector: SectorId,
}
impl ClusterReader {
    pub fn new(device: ArcMutex<CachedPartition>, start_sector: SectorId) -> Self {
        Self {
            final_sector: start_sector + SectorId(device.sectors_per_cluster),
            device,
            current_sector: start_sector,
            offset_byte_in_current_sector: 0,
        }
    }
    fn new_with_offset(
        device: ArcMutex<CachedPartition>,
        start_sector: SectorId,
        offset_byte_in_current_sector: usize,
    ) -> Self {
        Self {
            final_sector: start_sector + SectorId(device.sectors_per_cluster),
            device,
            current_sector: start_sector,
            offset_byte_in_current_sector,
        }
    }
    pub(crate) fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let cluster_is_over = self.current_sector >= self.final_sector;
        if cluster_is_over || buf.is_empty() {
            return Ok(0);
        }

        let mut total_amount_read = 0;
        let buf_len = buf.len();
        // Until buffer is full or I have read the whole cluster:
        while total_amount_read < buf_len && self.current_sector < self.final_sector {
            debug!(
                "Cluster reader, current sector: {current_sector}, Reading starting from {reading_start}",
                reading_start = total_amount_read + self.offset_byte_in_current_sector,
                current_sector = self.current_sector,
            );

            let space_left_in_current_sector =
                self.device.sector_size - self.offset_byte_in_current_sector;
            let amount_read = self.device.clone().read_sector_offset(
                self.current_sector,
                self.offset_byte_in_current_sector,
                &mut buf[total_amount_read..core::cmp::min(buf_len, space_left_in_current_sector)],
            )?;

            debug!(
                "ClusterReader: Current Sector: {}, Amount read: {}",
                self.current_sector, amount_read
            );
            total_amount_read += amount_read;
            self.offset_byte_in_current_sector += amount_read;

            assert!(self.offset_byte_in_current_sector <= self.device.sector_size);

            if self.offset_byte_in_current_sector == self.device.sector_size {
                self.current_sector = SectorId(self.current_sector.0 + 1);
                self.offset_byte_in_current_sector = 0;
            }
        }
        debug!(
            "Done filling the buffer. {}, {}. Final sector: {}.",
            total_amount_read < buf.len(),
            self.current_sector < self.final_sector,
            self.final_sector,
        );

        Ok(total_amount_read)
    }
}

/// this implements and encapsulates the logic needed to traverse
/// cluster chains, by reading the FAT table.
pub(crate) struct ClusterChainReader {
    pub device: ArcMutex<CachedPartition>,
    pub current_cluster: Option<ClusterId>,
    cluster_reader: ClusterReader,
    pub(crate) last_cluster_read: ClusterId,
}
impl ClusterChainReader {
    pub fn new(device: ArcMutex<CachedPartition>, cluster_to_read: ClusterId) -> Self {
        let current_sector = device.cluster_to_sector(cluster_to_read);
        let cluster_reader = ClusterReader::new(device.clone(), current_sector);

        Self {
            current_cluster: Some(cluster_to_read),
            cluster_reader,
            device,
            last_cluster_read: cluster_to_read,
        }
    }
    fn next_cluster(&self) -> Result<Option<ClusterId>> {
        if self.current_cluster.is_none() {
            return Ok(None);
        }
        fat_table::next_cluster(self.current_cluster.unwrap(), self.device.clone())
    }

    /// Assumptions: offset less then this object's size.
    pub fn seek(&mut self, offset: usize) -> Result<()> {
        // Calculate in which cluster this offset falls:
        let cluster_size = self.device.sectors_per_cluster as usize * self.device.sector_size;
        let cluster_offset = (offset as f64 / cluster_size as f64) as usize; // TODO: check if it's going to floor. apparently floor was removed from core?!

        // Calculate in which sector this offset falls:
        let sector_offset =
            offset / self.device.sector_size % self.device.sectors_per_cluster as usize;

        // Finally, calculate the offset in the selected sector:
        let offset_in_sector = offset % self.device.sector_size;
        info!(
            "Offset: {}, cluster_offset: {}, sector offset: {}, offset in sector: {}",
            offset, cluster_offset, sector_offset, offset_in_sector
        );
        for _ in 0..cluster_offset {
            self.current_cluster = self.next_cluster()?;
        }
        let start_sector = self.device.cluster_to_sector(self.current_cluster.unwrap())
            + SectorId(sector_offset as u32);

        self.cluster_reader =
            ClusterReader::new_with_offset(self.device.clone(), start_sector, offset_in_sector);

        Ok(())
    }

    fn cluster_reader_builder(&self) -> ClusterReader {
        let start_sector = self.device.cluster_to_sector(self.current_cluster.unwrap());
        ClusterReader::new(self.device.clone(), start_sector)
    }
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
}
