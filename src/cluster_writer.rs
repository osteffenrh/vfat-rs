use crate::{
    error, fat_reader, ArcMutex, BlockDevice, CachedPartition, ClusterId, MutexTrait, SectorId,
    VfatFS,
};
use log::{debug, info};

#[derive(Clone)]
pub struct ClusterWriter {
    pub device: ArcMutex<CachedPartition>,
    pub sector_size: usize,
    // TODO: remove, it's only used to calculate current sector
    pub cluster_start: SectorId,
    pub current_sector: SectorId,
    /// Offset in current_sector. In case buf.len()%sector_size != 0, this sector is not full read.
    /// The next read call will start from this offset.
    pub offset_byte_in_current_sector: usize,
    // TODO: remove, it's only used to calculate final sector
    pub sectors_per_cluster: u32,
    final_sector: SectorId,
}

impl ClusterWriter {
    /// Offset sector in cluster: it's current sector.
    pub fn new_offset(
        device: ArcMutex<CachedPartition>,
        cluster_start: SectorId,
        // TODO: rename to current_sector
        offset_sector_in_cluster: SectorId,
        sectors_per_cluster: u32,
        sector_size: usize,
        offset_byte_in_current_sector: usize,
    ) -> Self {
        Self {
            device,
            sector_size,
            offset_byte_in_current_sector,
            cluster_start,
            current_sector: cluster_start + offset_sector_in_cluster,
            sectors_per_cluster,
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
}

impl binrw::io::Write for ClusterWriter {
    fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, binrw::io::Error> {
        if self.is_over() || buf.is_empty() {
            return Ok(0);
        }
        let mut total_written = 0;
        while total_written < buf.len() && !self.is_over() {
            info!("CW: Total written: {}", total_written);
            info!(
                "CW: Current sector: {}, offset_byte: {}",
                self.current_sector, self.offset_byte_in_current_sector
            );
            let mut mutex = self.device.as_ref();
            let amount_written = mutex
                .lock(|device| {
                    device.write_sector_offset(
                        self.current_sector,
                        self.offset_byte_in_current_sector,
                        &buf[total_written..],
                    )
                })
                .map_err(|err| binrw::io::ErrorKind::Other)?; // TODO: fix error type
            info!("CW: amount written: {}", amount_written);

            total_written += amount_written;
            self.offset_byte_in_current_sector += amount_written;
            assert!(self.offset_byte_in_current_sector <= self.sector_size);

            // FIXME: Is this right?
            if self.offset_byte_in_current_sector == self.sector_size {
                info!("Sector is finished, going to switch sector...");
                self.current_sector = SectorId(self.current_sector.0 + 1);
                self.offset_byte_in_current_sector = 0;
            }
        }
        info!("CW: Written in total: {}", total_written);
        Ok(total_written)
    }

    fn flush(&mut self) -> core::result::Result<(), binrw::io::Error> {
        let mut mutex = self.device.as_ref();
        Ok(mutex
            .lock(|dev| dev.flush())
            .map_err(|err| binrw::io::ErrorKind::Other)?) // TODO: fix error type.
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
    pub fn new_w_offset(
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
    pub fn new(
        vfat_filesystem: VfatFS,
        start_cluster: ClusterId,
        offset_sector_in_cluster: SectorId,
    ) -> Self {
        let cluster_start = vfat_filesystem.cluster_to_sector(start_cluster);
        let cluster_writer = ClusterWriter::new(
            vfat_filesystem.device.clone(),
            cluster_start,
            offset_sector_in_cluster,
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
    pub fn seek(&mut self, offset: usize) -> error::Result<()> {
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
            "Offset: {}, cluster_offset: {}, sector offset: {}, offset in sector: {}",
            offset, cluster_offset, sector_offset, offset_in_sector
        );
        for _ in 0..cluster_offset {
            // Allocates cluster if needed:
            self.current_cluster = self.next_cluster()?;
        }

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
    fn next_cluster(&mut self) -> error::Result<Option<ClusterId>> {
        if self.current_cluster.is_none() {
            return Ok(None);
        }
        fat_reader::next_cluster(
            self.current_cluster.unwrap(),
            self.vfat_filesystem.sector_size,
            self.vfat_filesystem.device.clone(),
            self.vfat_filesystem.fat_start_sector,
        )
        .or_else(|_| {
            self.vfat_filesystem
                .allocate_cluster_to_chain(self.current_cluster.unwrap())
                .map(Option::from)
        })
    }
}

impl binrw::io::Write for ClusterChainWriter {
    fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, binrw::io::Error> {
        if self.current_cluster.is_none() || buf.is_empty() {
            info!(
                "Current cluster is: {:?}. Buf len: {}. Returning...",
                self.current_cluster,
                buf.len()
            );
            return Ok(0);
        }
        assert!(
            self.current_cluster.filter(|id| id.0 != 0).is_some(),
            "current cluster is ClusterId(0)."
        );
        debug!("CCW: Current cluster: {:?}", self.current_cluster);

        let mut amount_written = 0;
        while amount_written < buf.len() && self.current_cluster.is_some() {
            let current_amount_written = self
                .cluster_writer
                .write(&buf[amount_written..])
                .map_err(|err| binrw::io::ErrorKind::Other)?; // TODO: fix error type.
            amount_written += current_amount_written;
            if current_amount_written == 0 {
                self.current_cluster = self
                    .next_cluster()
                    .map_err(|err| binrw::io::ErrorKind::Other)?; // TODO: fix error type.
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

    fn flush(&mut self) -> core::result::Result<(), binrw::io::Error> {
        let mut mutex = self.vfat_filesystem.device.as_ref();
        Ok(mutex
            .lock(|device| device.flush())
            .map_err(|err| binrw::io::ErrorKind::Other)?) // TODO: fix error type.
    }
}
