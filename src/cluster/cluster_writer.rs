use crate::{error::Result, fat_table, ClusterId, SectorId, VfatFS};
use log::{debug, info};

pub struct ClusterChainWriter {
    vfat_fs: VfatFS,
    current_cluster: ClusterId,
    current_sector: SectorId,
    /// Offset in current_sector. In case buf.len()%sector_size != 0, this sector is not full read.
    /// The next read call will start from this offset.
    offset_byte_in_current_sector: usize,
}

impl ClusterChainWriter {
    pub(crate) fn new_w_offset(
        vfat_fs: VfatFS,
        start_cluster: ClusterId,
        offset_sector_in_cluster: SectorId,
        offset_in_sector: usize,
    ) -> Self {
        let cluster_start = vfat_fs.device.cluster_to_sector(start_cluster);
        Self {
            offset_byte_in_current_sector: offset_in_sector,
            current_sector: cluster_start + offset_sector_in_cluster,
            current_cluster: start_cluster,
            vfat_fs,
        }
    }

    /// start_sector: start on a different sector other then the one at beginning of the cluster.
    pub(crate) fn new(vfat_fs: VfatFS, start_cluster: ClusterId) -> Self {
        Self::new_w_offset(vfat_fs, start_cluster, SectorId::from(0), 0)
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
        self.offset_byte_in_current_sector = offset % self.vfat_fs.device.sector_size;

        for _ in 0..cluster_offset {
            self.current_cluster = self.next_cluster_alloc()?;
        }

        self.current_sector = self.vfat_fs.device.cluster_to_sector(self.current_cluster)
            + SectorId(sector_offset as u32);

        Ok(())
    }

    fn next_cluster_alloc(&mut self) -> Result<ClusterId> {
        let ret = fat_table::next_cluster(self.current_cluster, self.vfat_fs.device.clone())?;

        Ok(match ret {
            None => self
                .vfat_fs
                .allocate_cluster_to_chain(self.current_cluster)?,
            Some(r) => r,
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        assert_ne!(
            self.current_cluster,
            ClusterId::new(0),
            "current cluster is ClusterId(0)."
        );

        let mut amount_written = 0;
        while amount_written < buf.len() {
            let current_amount_written = self.write_cluster(&buf[amount_written..])?;
            amount_written += current_amount_written;
            if current_amount_written == 0 {
                self.current_cluster = self.next_cluster_alloc()?;
                self.current_sector = self.vfat_fs.device.cluster_to_sector(self.current_cluster);
                self.offset_byte_in_current_sector = 0;
            }
        }
        debug!("CWW: Amount writen: {}", amount_written);
        Ok(amount_written)
    }

    fn write_cluster(&mut self, buf: &[u8]) -> core::result::Result<usize, binrw::io::Error> {
        if self.is_over() || buf.is_empty() {
            return Ok(0);
        }
        let mut total_written = 0;
        while total_written < buf.len() && !self.is_over() {
            let space_left_in_current_sector =
                self.vfat_fs.device.sector_size - self.offset_byte_in_current_sector;
            let amount_written = self.vfat_fs.device.clone().write_sector_offset(
                self.current_sector,
                self.offset_byte_in_current_sector,
                &buf[total_written
                    ..core::cmp::min(total_written + space_left_in_current_sector, buf.len())],
            )?;
            total_written += amount_written;
            self.offset_byte_in_current_sector += amount_written;
            assert!(self.offset_byte_in_current_sector <= self.vfat_fs.device.sector_size);

            if self.offset_byte_in_current_sector == self.vfat_fs.device.sector_size {
                // Sector is finished, let's go to the next one
                self.current_sector = SectorId(self.current_sector + 1);
                self.offset_byte_in_current_sector = 0;
            }
        }
        Ok(total_written)
    }

    fn is_over(&self) -> bool {
        let cluster_start = self.vfat_fs.device.cluster_to_sector(self.current_cluster);
        let final_sector = SectorId(self.vfat_fs.device.sectors_per_cluster) + cluster_start;
        self.current_sector >= final_sector
    }
}
