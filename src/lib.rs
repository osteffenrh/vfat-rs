//! ntfs-rs is a simple ntfs implementation in Rust.
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![deny(unaligned_references)]
//#![deny(missing_docs)]
//#![deny(unsafe_code)]
// to remove:
//#![allow(unused_variables)]
//#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

extern crate alloc;
extern crate core;

use alloc::sync::Arc;

use api::directory_entry::{
    Attributes, RegularDirectoryEntry, UnknownDirectoryEntry, VfatDirectoryEntry,
};
pub use api::EntryType;
pub use api::{Directory, Metadata, VfatEntry, VfatMetadataTrait};
pub(crate) use cache::CachedPartition;
pub use device::BlockDevice;
#[cfg(feature = "std")]
pub use device::FilebackedBlockDevice;
pub use error::{Result, VfatRsError};
pub(crate) use formats::cluster_id::ClusterId;
#[cfg(not(feature = "std"))]
pub use formats::path::Path;
#[cfg(feature = "std")]
pub use std::path::PathBuf as Path;

pub use formats::sector_id::SectorId;
pub use vfat::VfatFS;

mod api;
mod cache;
mod cluster;
mod device;
/// NtfsRs error definitions
mod error;
mod fat_table;
mod formats;
pub mod io;
mod macros;
/// A simple Master Booot Record implementation
pub mod mbr;
mod vfat;

const EBPF_VFAT_MAGIC: u8 = 0x28;
const EBPF_VFAT_MAGIC_ALT: u8 = 0x29;

/// Why Arc? Because CachedPartition owns the block device. And
/// Vfat needs to be cloned, and potentially we could send references across threads.
type ArcMutex<CachedPartition> = Arc<CachedPartition>;
