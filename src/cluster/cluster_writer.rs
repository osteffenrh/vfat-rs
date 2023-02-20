use crate::{error::Result, fat_table, ArcMutex, CachedPartition, ClusterId, SectorId, VfatFS};
use log::{debug, info};

#[derive(Clone)]
struct ClusterWriter {
    pub device: ArcMutex<CachedPartition>,
    pub sector_size: usize,
    pub current_sector: SectorId,
    /// Offset in current_sector. In case buf.len()%sector_size != 0, this sector is not full read.
    /// The next read call will start from this offset.
    pub offset_byte_in_current_sector: usize,
    final_sector: SectorId,
}

impl ClusterWriter {
    /// Offset sector in cluster: it's current sector.
    pub fn new_offset(
        device: ArcMutex<CachedPartition>,
        cluster_start: SectorId,
        offset_sector_in_cluster: SectorId,
        sectors_per_cluster: u32,
        sector_size: usize,
        offset_byte_in_current_sector: usize,
    ) -> Self {
        Self {
            device,
            sector_size,
            offset_byte_in_current_sector,
            current_sector: cluster_start + offset_sector_in_cluster,
            final_sector: SectorId(sectors_per_cluster) + cluster_start,
        }
    }
    pub fn new(
        device: ArcMutex<CachedPartition>,
        cluster_start: SectorId,
        offset_sector_in_cluster: SectorId,
        sectors_per_cluster: u32,
        sector_size: usize,
    ) -> Self {
        Self::new_offset(
            device,
            cluster_start,
            offset_sector_in_cluster,
            sectors_per_cluster,
            sector_size,
            0,
        )
    }
    fn is_over(&self) -> bool {
        self.current_sector >= self.final_sector
    }

    pub fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, binrw::io::Error> {
        if self.is_over() || buf.is_empty() {
            return Ok(0);
        }
        let mut total_written = 0;
        while total_written < buf.len() && !self.is_over() {
            debug!("CW: Total written: {}", total_written);
            let space_left_in_current_sector =
                self.sector_size - self.offset_byte_in_current_sector;
            let amount_written = self.device.clone().write_sector_offset(
                self.current_sector,
                self.offset_byte_in_current_sector,
                &buf[total_written
                    ..core::cmp::min(total_written + space_left_in_current_sector, buf.len())],
            )?;
            total_written += amount_written;
            self.offset_byte_in_current_sector += amount_written;
            debug!(
                "CW: Amount written: {}, total written: {}, Current sector: {}, offset_byte: {}, sector size: {}",
                amount_written,
                total_written,
                self.current_sector,
                self.offset_byte_in_current_sector,
                self.sector_size
            );
            assert!(self.offset_byte_in_current_sector <= self.sector_size);

            if self.offset_byte_in_current_sector == self.sector_size {
                debug!("Sector is finished, going to switch sector...");
                self.current_sector = SectorId(self.current_sector.0 + 1);
                self.offset_byte_in_current_sector = 0;
            }
        }
        debug!("CW: Written in total: {}", total_written);
        Ok(total_written)
    }

    fn _flush(&mut self) -> core::result::Result<(), binrw::io::Error> {
        Ok(self.device.flush()?)
    }
}

#[derive(Clone)]
pub struct ClusterChainWriter {
    vfat_filesystem: VfatFS,
    cluster_writer: ClusterWriter,
    //TODO: no need to wrap in an option
    current_cluster: Option<ClusterId>,
}
impl ClusterChainWriter {
    pub(crate) fn new_w_offset(
        vfat_filesystem: VfatFS,
        start_cluster: ClusterId,
        offset_sector_in_cluster: SectorId,
        offset_in_sector: usize,
    ) -> Self {
        let cluster_writer = ClusterWriter::new_offset(
            vfat_filesystem.device.clone(),
            vfat_filesystem.cluster_to_sector(start_cluster),
            offset_sector_in_cluster,
            vfat_filesystem.sectors_per_cluster,
            vfat_filesystem.sector_size,
            offset_in_sector,
        );
        Self {
            vfat_filesystem,
            current_cluster: Some(start_cluster),
            cluster_writer,
        }
    }

    ///
    /// start_sector: start on a different sector other then the one at beginning of the cluster.
    pub(crate) fn new(vfat_filesystem: VfatFS, start_cluster: ClusterId) -> Self {
        let cluster_start = vfat_filesystem.cluster_to_sector(start_cluster);
        let cluster_writer = ClusterWriter::new(
            vfat_filesystem.device.clone(),
            cluster_start,
            SectorId(0),
            vfat_filesystem.sectors_per_cluster,
            vfat_filesystem.sector_size,
        );
        Self {
            vfat_filesystem,
            current_cluster: Some(start_cluster),
            cluster_writer,
        }
    }
    fn cluster_writer_builder(&self) -> ClusterWriter {
        let start_sector = self
            .vfat_filesystem
            .cluster_to_sector(self.current_cluster.unwrap());
        ClusterWriter::new(
            self.vfat_filesystem.device.clone(),
            start_sector,
            SectorId(0),
            self.vfat_filesystem.sectors_per_cluster,
            self.vfat_filesystem.sector_size,
        )
    }
    // TODO: move to impl Seek trait.
    pub fn seek(&mut self, offset: usize) -> Result<()> {
        // Calculate in which cluster this offset falls:
        let cluster_size =
            self.vfat_filesystem.sectors_per_cluster as usize * self.vfat_filesystem.sector_size;
        let cluster_offset = (offset as f64 / cluster_size as f64) as usize; //TODO: check it's floor()

        // Calculate in which sector this offset falls:
        let sector_offset = offset / self.vfat_filesystem.sector_size
            % self.vfat_filesystem.sectors_per_cluster as usize;

        // Finally, calculate the offset in the selected sector:
        let offset_in_sector = offset % self.vfat_filesystem.sector_size;
        info!(
            "Offset: {}, cluster_offset: {}, sector offset: {}, offset in sector: {}, current cluster: {:?}",
            offset, cluster_offset, sector_offset, offset_in_sector, self.current_cluster
        );
        for _ in 0..cluster_offset {
            // Allocates cluster if needed:
            self.current_cluster = self.next_cluster()?;
        }
        info!("Current cluster: {:?}", self.current_cluster);
        self.cluster_writer = ClusterWriter::new_offset(
            self.vfat_filesystem.device.clone(),
            self.vfat_filesystem
                .cluster_to_sector(self.current_cluster.unwrap()),
            SectorId(sector_offset as u32),
            self.vfat_filesystem.sectors_per_cluster,
            self.vfat_filesystem.sector_size,
            offset_in_sector,
        );

        Ok(())
    }
    fn next_cluster(&mut self) -> Result<Option<ClusterId>> {
        if self.current_cluster.is_none() {
            return Ok(None);
        }
        let mut ret = fat_table::next_cluster(
            self.current_cluster.unwrap(),
            self.vfat_filesystem.device.clone(),
        )?;
        if ret.is_none() {
            ret = Some(
                self.vfat_filesystem
                    .allocate_cluster_to_chain(self.current_cluster.unwrap())?,
            );
        }
        Ok(ret)
    }
    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.current_cluster.is_none() || buf.is_empty() {
            info!(
                "Current cluster is: {:?}. Buf len: {}. Returning...",
                self.current_cluster,
                buf.len()
            );
            return Ok(0);
        }
        assert!(
            self.current_cluster
                .filter(|id| u32::from(*id) != 0)
                .is_some(),
            "current cluster is ClusterId(0)."
        );
        debug!(
            "CCW: Current cluster: {:?}, offset: {}",
            self.current_cluster, self.cluster_writer.offset_byte_in_current_sector
        );

        let mut amount_written = 0;
        while amount_written < buf.len() && self.current_cluster.is_some() {
            let current_amount_written = self.cluster_writer.write(&buf[amount_written..])?;
            amount_written += current_amount_written;
            if current_amount_written == 0 {
                self.current_cluster = self.next_cluster()?;
                if self.current_cluster.is_some() {
                    // If there is another cluster in the chain,
                    // create a new cluster writer.
                    self.cluster_writer = self.cluster_writer_builder();
                }
            }
        }
        info!("CWW: Amount writen: {}", amount_written);
        Ok(amount_written)
    }

    fn _flush(&mut self) -> Result<()> {
        self.vfat_filesystem.device.flush()
    }
}
