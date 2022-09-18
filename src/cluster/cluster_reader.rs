use log::{debug, info};

use crate::cache::CachedPartition;
use crate::{fat_reader, ArcMutex, BlockDevice, ClusterId, Result, SectorId};

pub(crate) fn cluster_to_sector(
    cluster: ClusterId,
    sectors_per_cluster: u32,
    data_start_sector: SectorId,
) -> SectorId {
    let selected_sector = (u32::from(cluster) - 2) * sectors_per_cluster;
    let sect = data_start_sector + selected_sector;
    SectorId(sect)
}

/// A reader for the content of a single FAT cluster
/// It reads from the beginning of the cluster.
/// It's not thread-safe and should not be shared.
/// Given a buffer buf, it will try to read as much sectors as it can in order to fill the buffer.
#[derive(Clone)]
struct ClusterReader {
    pub device: ArcMutex<CachedPartition>,
    pub sector_size: usize,
    pub current_sector: SectorId,
    /// Offset in current_sector. In case buf.len()%sector_size != 0, this sector is not full read.
    /// The next read call will start from this offset.
    pub offset_byte_in_current_sector: usize,
    // TODO: rm, it's only used to set final_sector.
    pub sectors_per_cluster: u32,
    final_sector: SectorId,
}
impl ClusterReader {
    pub fn new(
        device: ArcMutex<CachedPartition>,
        start_sector: SectorId,
        sectors_per_cluster: u32,
        sector_size: usize,
    ) -> Self {
        Self {
            device,
            current_sector: start_sector,
            offset_byte_in_current_sector: 0,
            sector_size,
            sectors_per_cluster,
            final_sector: start_sector + SectorId(sectors_per_cluster),
        }
    }
    fn new_with_offset(
        device: ArcMutex<CachedPartition>,
        start_sector: SectorId,
        sectors_per_cluster: u32,
        sector_size: usize,
        offset_byte_in_current_sector: usize,
    ) -> Self {
        Self {
            device,
            current_sector: start_sector,
            offset_byte_in_current_sector,
            sector_size,
            sectors_per_cluster,
            final_sector: start_sector + SectorId(sectors_per_cluster),
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
                self.sector_size - self.offset_byte_in_current_sector;
            let mut dev_lock = self.device.lock();
            let amount_read = (*dev_lock).read_sector_offset(
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

            assert!(self.offset_byte_in_current_sector <= self.sector_size);

            if self.offset_byte_in_current_sector == self.sector_size {
                self.current_sector = SectorId(self.current_sector.0 + 1);
                self.offset_byte_in_current_sector = 0;
            }
        }
        debug!(
            "Done filling the buffer. {}, {}. Final sector: {}. Sectors per cluster: {}",
            total_amount_read < buf.len(),
            self.current_sector < self.final_sector,
            self.final_sector,
            self.sectors_per_cluster
        );

        Ok(total_amount_read)
    }
}

/// this implements and encapsulates the logic needed to traverse
/// cluster chains, by reading the FAT table.
#[derive(Clone)]
pub(crate) struct ClusterChainReader {
    pub device: ArcMutex<CachedPartition>,
    pub sector_size: usize,
    pub sectors_per_cluster: u32,
    pub data_start_sector: SectorId,
    pub current_cluster: Option<ClusterId>,
    cluster_reader: ClusterReader,
    pub(crate) last_cluster_read: ClusterId,
}
impl ClusterChainReader {
    pub fn new(
        device: ArcMutex<CachedPartition>,
        sector_size: usize,
        sectors_per_cluster: u32,
        cluster_to_read: ClusterId,
        data_start_sector: SectorId,
    ) -> Self {
        let current_sector =
            cluster_to_sector(cluster_to_read, sectors_per_cluster, data_start_sector);
        let cluster_reader = ClusterReader::new(
            device.clone(),
            current_sector,
            sectors_per_cluster,
            sector_size,
        );

        Self {
            sector_size,
            sectors_per_cluster,
            data_start_sector,
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
        fat_reader::next_cluster(self.current_cluster.unwrap(), self.device.clone())
    }

    /// Assumptions: offset less then this object's size.
    pub fn seek(&mut self, offset: usize) -> Result<()> {
        // Calculate in which cluster this offset falls:
        let cluster_size = self.sectors_per_cluster as usize * self.sector_size;
        let cluster_offset = (offset as f64 / cluster_size as f64) as usize; // TODO: check if it's going to floor. apparently floor was removed from core?!

        // Calculate in which sector this offset falls:
        let sector_offset = offset / self.sector_size % self.sectors_per_cluster as usize;

        // Finally, calculate the offset in the selected sector:
        let offset_in_sector = offset % self.sector_size;
        info!(
            "Offset: {}, cluster_offset: {}, sector offset: {}, offset in sector: {}",
            offset, cluster_offset, sector_offset, offset_in_sector
        );
        for _ in 0..cluster_offset {
            self.current_cluster = self.next_cluster()?;
        }
        let start_sector = cluster_to_sector(
            self.current_cluster.unwrap(),
            self.sectors_per_cluster,
            self.data_start_sector,
        ) + SectorId(sector_offset as u32);

        self.cluster_reader = ClusterReader::new_with_offset(
            self.device.clone(),
            start_sector,
            self.sectors_per_cluster,
            self.sector_size,
            offset_in_sector,
        );

        Ok(())
    }

    fn cluster_reader_builder(&self) -> ClusterReader {
        let start_sector = cluster_to_sector(
            self.current_cluster.unwrap(),
            self.sectors_per_cluster,
            self.data_start_sector,
        );
        ClusterReader::new(
            self.device.clone(),
            start_sector,
            self.sectors_per_cluster,
            self.sector_size,
        )
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
