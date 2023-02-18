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

use spin::mutex::SpinMutex;

pub(crate) use cache::CachedPartition;
pub use device::BlockDevice;
pub use error::{Result, VfatRsError};
use fat_table::fat_entry::FatEntry;
use fat_table::{fat_reader, fat_writer};
pub(crate) use formats::cluster_id::ClusterId;
pub use formats::path::Path;
pub use formats::sector_id::SectorId;
use os_interface::directory_entry::{
    Attributes, RegularDirectoryEntry, UnknownDirectoryEntry, VfatDirectoryEntry,
};
pub use os_interface::EntryType;
pub use os_interface::{Directory, Metadata, VfatEntry, VfatMetadataTrait};
pub use vfat::VfatFS;

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

const EBPF_VFAT_MAGIC: u8 = 0x28;
const EBPF_VFAT_MAGIC_ALT: u8 = 0x29;

type ArcMutex<T> = Arc<SpinMutex<T>>;
