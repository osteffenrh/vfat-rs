//! ntfs-rs is a simple ntfs implementation in Rust.
//#![feature(bool_to_option)]
#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]
#![deny(unaligned_references)]
#![deny(missing_docs)]

extern crate alloc;

/// NtfsRs error definitions
mod error;
mod macros;
/// A simple Master Booot Record implementation
pub mod mbr;

/// A sector ID for a Block device.
pub struct SectorId(u64);

/// A block device is a computer data storage device that supports reading
/// and (optionally) writing data in fixed-size blocks, sectors, or clusters.
/// These blocks are generally 512 bytes or a multiple thereof in size.
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
        self.read_sector_offset(sector, 0, buf)
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
        Self: Sized;
}
