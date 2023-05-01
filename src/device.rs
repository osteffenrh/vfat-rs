#[cfg(feature = "std")]
use crate::io::{Read, Seek};
use crate::{error, SectorId};
#[cfg(feature = "std")]
use log::debug;

/// A block device is a computer data storage device that supports reading
/// and (optionally) writing data in fixed-size blocks, sectors, or clusters.
/// These blocks are generally 512 bytes or a multiple thereof in size.
/// TODO: move _offset functions to cachedpartition only.
pub trait BlockDevice {
    /// Sector size in bytes.
    fn sector_size(&self) -> usize {
        512
    }

    /// Read sector `n` in `buf`, up to min(self.sector_size() and buf.size()).
    /// Returns the amount of the bytes read.
    ///
    /// Needs to be mutable because, for instance we might
    /// need to use seek to move the pointer on the file
    fn read_sector(&mut self, sector: SectorId, buf: &mut [u8]) -> error::Result<usize> {
        self.read_sector_offset(sector, 0, buf) //TODO: this is wrong. it should keep track of offset somewhere.
    }

    /// Read a sector starting from an offset.
    fn read_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        buf: &mut [u8],
    ) -> error::Result<usize>;

    /// Write an entire sector.
    fn write_sector(&mut self, sector: SectorId, buf: &[u8]) -> error::Result<usize> {
        self.write_sector_offset(sector, 0, buf)
    }

    /// write start from an offset in a sector
    fn write_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        buf: &[u8],
    ) -> error::Result<usize>;

    /// A human readable name for this device
    fn get_canonical_name() -> &'static str
    where
        Self: Sized,
    {
        "Block Device"
    }
}

/// FilebackedBlockDevice is an implementation of BlockDevice backed by
/// std::fs::File. It's a simple way to explore a vfat fs on a file.
#[cfg(feature = "std")]
pub struct FilebackedBlockDevice {
    pub image: std::fs::File,
}

#[cfg(feature = "std")]
impl BlockDevice for FilebackedBlockDevice {
    fn sector_size(&self) -> usize {
        512
    }

    fn read_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        mut buf: &mut [u8],
    ) -> crate::Result<usize> {
        use core::cmp::min;
        let max_read = min(buf.len(), self.sector_size());
        let mut temp_buf = vec![0; max_read];
        let final_destination = sector.0 as u64 * self.sector_size() as u64 + offset as u64;
        debug!(
            "Sector: {}, offset: {}, finaldest: {}",
            sector.0 as u64 * self.sector_size() as u64,
            offset,
            final_destination
        );
        self.image
            .seek(std::io::SeekFrom::Start(final_destination))
            .expect("Impossible to seek to the sector");

        self.image
            .read_exact(temp_buf.as_mut_slice())
            .expect("Impossible to read from image");
        debug!("done reading read_sector_offset...");
        use crate::io::Write;
        buf.write(temp_buf.as_mut_slice()).map_err(Into::into)
    }

    fn write_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        buf: &[u8],
    ) -> crate::Result<usize> {
        use std::io::Write;
        let final_destination = sector.0 as u64 * self.sector_size() as u64 + offset as u64;
        debug!(
            "Seeking to : sector: {}, sector_size: {}, offset: {}, final destination: {} ",
            sector,
            self.sector_size(),
            offset,
            final_destination
        );
        self.image
            .seek(std::io::SeekFrom::Start(final_destination))
            .expect("Error seek");
        debug!("Writing the buffer to the image..");
        self.image.write_all(buf).expect("Write sector");
        debug!("Written: {}", buf.len());
        self.image.flush().unwrap();
        Ok(buf.len())
    }

    fn get_canonical_name() -> &'static str
    where
        Self: Sized,
    {
        "FileBasedBlockDevice"
    }
}
