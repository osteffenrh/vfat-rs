//! ntfs-rs is a simple ntfs implementation in Rust.
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![deny(unaligned_references)]
//#![deny(missing_docs)]
//#![deny(unsafe_code)]
// to remove:
//#![allow(unused_variables)]
//#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::blocks_in_if_conditions)]

extern crate alloc;
extern crate core;

use alloc::sync::Arc;
use spin::mutex::SpinMutex;

mod cache;
mod cluster;
mod device;
/// NtfsRs error definitions
mod error;
mod fat_table;
mod formats;
mod macros;
/// A simple Master Booot Record implementation
pub mod mbr;
mod os_interface;
mod vfat;

pub use crate::device::BlockDevice;
use crate::error::VfatRsError::FreeClusterNotFound;
use crate::os_interface::directory_entry::{
    Attributes, RegularDirectoryEntry, UnknownDirectoryEntry, VfatDirectoryEntry,
};
pub use crate::os_interface::EntryType;
pub use cache::CachedPartition;
pub use formats::path::Path;

use crate::error::VfatRsError;
pub use crate::fat_table::fat_entry::{FatEntry, RawFatEntry};
use crate::fat_table::{fat_reader, fat_writer};
pub use crate::formats::cluster_id::ClusterId;
pub use crate::formats::sector_id::SectorId;
pub use crate::os_interface::{VfatDirectory, VfatEntry, VfatMetadata, VfatMetadataTrait};
pub use error::Result;
pub use vfat::VfatFS;

const EBPF_VFAT_MAGIC: u8 = 0x28;
const EBPF_VFAT_MAGIC_ALT: u8 = 0x29;

type ArcMutex<T> = Arc<SpinMutex<T>>;
