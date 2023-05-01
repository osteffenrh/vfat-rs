//! ntfs-rs is a simple ntfs implementation in Rust.
#![cfg_attr(not(any(test, feature = "std")), no_std)]
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

pub use traits::{TimeManagerNoop, TimeManagerTrait};
pub mod traits {
    use crate::api::timestamp::VfatTimestamp;
    use alloc::sync::Arc;
    use core::fmt::Debug;

    // An interface to the OS-owned timer. Needed for timestamping file creations and update.
    pub trait TimeManagerTrait: Debug {
        /// Get the current Unix timestamp in milliseconds.
        /// The number of seconds since January 1, 1970, 00:00:00 UTC
        fn get_current_timestamp(&self) -> u64;
        fn get_current_vfat_timestamp(&self) -> VfatTimestamp {
            let is_leap_year =
                |year| -> bool { (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 };
            const SECONDS_IN_MINUTE: u32 = 60;
            const SECONDS_IN_HOUR: u32 = 60 * SECONDS_IN_MINUTE;
            const SECONDS_IN_DAY: u32 = 24 * SECONDS_IN_HOUR;

            let mut remaining_seconds = self.get_current_timestamp() as u32;

            let mut days_since_1970 = remaining_seconds / SECONDS_IN_DAY;
            remaining_seconds %= SECONDS_IN_DAY;

            let mut year = 1970u32;
            let mut day_count;

            loop {
                day_count = if is_leap_year(year) { 366 } else { 365 };
                if days_since_1970 >= day_count {
                    days_since_1970 -= day_count;
                    year += 1;
                } else {
                    break;
                }
            }

            let mut month = 1u32;
            let days_in_month = [
                31,
                28 + (is_leap_year(year) as u32),
                31,
                30,
                31,
                30,
                31,
                31,
                30,
                31,
                30,
                31,
            ];

            while days_since_1970 >= days_in_month[(month - 1) as usize] {
                days_since_1970 -= days_in_month[(month - 1) as usize];
                month += 1;
            }

            let day = days_since_1970 + 1;
            let hour = remaining_seconds / SECONDS_IN_HOUR;
            remaining_seconds %= SECONDS_IN_HOUR;
            let minute = remaining_seconds / SECONDS_IN_MINUTE;
            let second = remaining_seconds % SECONDS_IN_MINUTE;

            let mut timestamp = VfatTimestamp::new(0);

            timestamp
                // 1980 is the min in vfat timestamps.
                .set_year(year)
                .set_value(month, VfatTimestamp::MONTH)
                .set_value(day, VfatTimestamp::DAY)
                .set_value(hour, VfatTimestamp::HOURS)
                .set_value(minute, VfatTimestamp::MINUTES)
                .set_seconds(second); // VFAT has a 2-second resolution

            timestamp
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct TimeManagerNoop {}
    impl TimeManagerNoop {
        pub fn new() -> Self {
            Default::default()
        }
        pub fn new_arc() -> Arc<Self> {
            Arc::new(Self {})
        }
    }
    impl TimeManagerTrait for TimeManagerNoop {
        fn get_current_timestamp(&self) -> u64 {
            0
        }
    }

    #[cfg(feature = "std")]
    #[derive(Clone, Debug)]
    pub struct TimeManagerChronos {}
    #[cfg(feature = "std")]
    impl TimeManagerChronos {
        pub(crate) fn new() -> Self {
            Self {}
        }
    }
    #[cfg(feature = "std")]
    impl TimeManagerTrait for TimeManagerChronos {
        fn get_current_timestamp(&self) -> u64 {
            use chrono::Utc;
            let now = Utc::now();
            let seconds_since_epoch: i64 = now.timestamp();
            // I guess it's an i64 because of underflow for dates before 1970
            seconds_since_epoch as u64
        }
    }
}
