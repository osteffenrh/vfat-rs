//! ntfs-rs is a simple ntfs implementation in Rust.
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![deny(unaligned_references)]
//#![deny(missing_docs)]
//#![deny(unsafe_code)]

// to remove:
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::blocks_in_if_conditions)]

extern crate alloc;

use crate::extended_bios_parameter_block::FullExtendedBIOSParameterBlock;
use alloc::sync::Arc;
use binrw::io::{Cursor, Error, Read};
use binrw::BinReaderExt;
use core::{fmt, mem};
use log::{debug, info};

mod api;
mod cache;
mod cluster_id;
mod cluster_reader;
mod cluster_writer;
mod device;
/// NtfsRs error definitions
mod error;
mod extended_bios_parameter_block;
mod fat_entry;
mod fat_reader;
mod fat_writer;
mod lock;
mod macros;
/// A simple Master Booot Record implementation
pub mod mbr;
mod os_interface;
pub mod sector_id;
mod timestamp;
mod traits;
mod utils;

pub use crate::cache::CachedPartition;
pub use crate::cluster_id::ClusterId;
pub use crate::device::BlockDevice;
use crate::error::BinRwErrorWrapper;
use crate::error::Error::{BinReadConvFailed, FreeClusterNotFound};
use crate::fat_entry::FatEntry;
pub use crate::fat_entry::RawFatEntry;
pub use crate::lock::{MutexTrait, NullLock};
use crate::os_interface::directory_entry::{
    Attributes, RegularDirectoryEntry, UnknownDirectoryEntry, VfatDirectoryEntry,
};
pub use crate::os_interface::EntryType;
pub use crate::os_interface::Path;
use crate::os_interface::{VfatDirectory, VfatEntry, VfatMetadata};
pub use error::Result;
use sector_id::SectorId;

type ArcMutex<T> = Arc<NullLock<T>>;

#[derive(Clone)]
pub struct VfatFS {
    pub device: ArcMutex<CachedPartition>,
    /// Sector of the file allocation table
    pub fat_start_sector: SectorId,
    /// First sector containing actual data - after all FAT tables.
    pub data_start_sector: SectorId,
    /// How many sectors are mapped to a single cluster
    pub sectors_per_cluster: u32,
    /// How many sectors each fat table uses.
    pub sectors_per_fat: u32,
    /// Id for the root_cluster
    pub root_cluster: ClusterId,
    /// End of chain marker
    pub eoc_marker: RawFatEntry,
    pub sector_size: usize,
}

impl fmt::Debug for VfatFS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VfatFilesystem")
    }
}

impl VfatFS {
    pub fn new<B: BlockDevice + 'static>(
        mut device: B,
        partition_start_sector: u32,
    ) -> error::Result<Self> {
        let full_ebpb = Self::read_fullebpb(&mut device, partition_start_sector)?;
        Self::new_with_ebpb(device, partition_start_sector, full_ebpb)
    }

    pub fn read_fullebpb<B: BlockDevice>(
        device: &mut B,
        start_sector: u32,
    ) -> error::Result<FullExtendedBIOSParameterBlock> {
        let mut buff = [0u8; 512];
        device.read_sector(start_sector.into(), &mut buff)?;
        Cursor::new(&buff)
            .read_ne()
            .map_err(BinRwErrorWrapper::from)
            .map_err(|err| BinReadConvFailed { source: err })
    }

    /// start_sector: Partition's start sector, or "Entry Offset Sector".
    pub fn new_with_ebpb<B: BlockDevice + 'static>(
        mut device: B,
        partition_start_sector: u32,
        full_ebpb: FullExtendedBIOSParameterBlock,
    ) -> error::Result<Self> {
        let fat_start_sector =
            (partition_start_sector + full_ebpb.bpb.reserved_sectors as u32).into();
        let fats_total_size = full_ebpb.extended.sectors_per_fat * full_ebpb.bpb.fat_amount as u32;
        let data_start_sector =
            fat_start_sector + fats_total_size as u32 + full_ebpb.sectors_occupied_by_all_fats();
        let data_start_sector = SectorId(data_start_sector);

        let sectors_per_cluster = full_ebpb.bpb.sectors_per_cluster as u32;
        let root_cluster = ClusterId::new(full_ebpb.extended.root_cluster);
        let eoc_marker = Self::read_end_of_chain_marker(&mut device, fat_start_sector)?;
        let sector_size = device.sector_size();
        let cached_partition = CachedPartition::new(device);
        let sectors_per_fat = full_ebpb.extended.sectors_per_fat;
        Ok(VfatFS {
            sector_size,
            device: Arc::new(NullLock::new(cached_partition)),
            fat_start_sector,
            data_start_sector,
            sectors_per_cluster,
            root_cluster,
            eoc_marker,
            sectors_per_fat,
        })
    }

    pub fn get_cluster_size(&self) -> u32 {
        self.sectors_per_cluster * self.sector_size as u32
    }

    fn read_end_of_chain_marker<B>(
        device: &mut B,
        fat_start_sector: SectorId,
    ) -> error::Result<RawFatEntry>
    where
        B: BlockDevice,
    {
        const FAT_ENTRY_SIZE: usize = core::mem::size_of::<RawFatEntry>();
        const ENTRIES_BUF_SIZE: usize = 1;
        const BUF_SIZE: usize = FAT_ENTRY_SIZE * ENTRIES_BUF_SIZE;
        let mut buf = [0; BUF_SIZE];
        device.read_sector(fat_start_sector, &mut buf).unwrap();
        let raw_entry: RawFatEntry = unsafe { mem::transmute(buf) };
        info!("End of chain marker: {:?}", raw_entry);
        Ok(raw_entry)
    }

    fn get_last_cluster_entry(&self) -> FatEntry {
        FatEntry::LastCluster(self.eoc_marker.0)
    }

    /// Converts a cluster (a FAT concept) to a sector (a BlockDevice concept).
    ///
    /// To do so, it uses some useful info from the BPB section.
    pub(crate) fn cluster_to_sector(&self, cluster: ClusterId) -> SectorId {
        let selected_sector =
            u32::from(cluster).saturating_sub(2) * self.sectors_per_cluster as u32;
        let sect = self.data_start_sector.0 as u32 + selected_sector as u32;
        SectorId(sect)
    }

    /// Find next free cluster
    pub fn find_free_cluster(&self) -> error::Result<Option<ClusterId>> {
        info!("Starting find free cluster routine");
        const FAT_ENTRY_SIZE: usize = core::mem::size_of::<RawFatEntry>();
        const ENTRIES_BUF_SIZE: usize = 512 / FAT_ENTRY_SIZE;
        const BUF_SIZE: usize = FAT_ENTRY_SIZE * ENTRIES_BUF_SIZE;
        let sectors_per_fat = self.sectors_per_fat;
        let fat_start_sector: u64 = self.fat_start_sector.into();
        for i in 0..sectors_per_fat {
            let mut buf = [0; BUF_SIZE];
            let mut mutex = self.device.as_ref();
            info!("reading sector: {}/{}", i, sectors_per_fat);
            mutex.lock(|device| {
                device
                    .read_sector(SectorId(fat_start_sector as u32 + i), &mut buf)
                    .unwrap();
            });
            let raw_entries: [RawFatEntry; ENTRIES_BUF_SIZE] = unsafe { mem::transmute(buf) };
            for (id, raw) in raw_entries.iter().enumerate() {
                let cid = (ENTRIES_BUF_SIZE as u32 * i) as u32 + id as u32;
                debug!("(cid: {:?}) Fat entry: {:?}", FatEntry::from(*raw), cid);
                if let FatEntry::Unused = FatEntry::from(*raw) {
                    debug!("Found an unused cluster with id: {}", cid);
                    return Ok(Some(ClusterId::new((ENTRIES_BUF_SIZE as u32 * i) + id as u32)));
                }
            }
        }
        Ok(None)
    }

    pub fn allocate_cluster_new_entry(&mut self) -> error::Result<ClusterId> {
        let status = self.get_last_cluster_entry();
        let free_cluster_id = self.find_free_cluster()?.ok_or(FreeClusterNotFound)?;
        info!("Found free cluster: {}", free_cluster_id);
        self.write_entry_in_vfat_table(free_cluster_id, status)?;
        Ok(free_cluster_id)
    }

    /// Finds a free clusters and updates the chain:
    ///  * previous cluster in the chain to point to the newly allocated one,
    /// * new clusterId added as final entry
    /// TODO: invert writes, first update head, and then allocate the cluster.
    pub fn allocate_cluster_to_chain(&self, head: ClusterId) -> error::Result<ClusterId> {
        info!("Allocating cluster to chain: {}", head);
        debug!("Head cluster: {}", head);
        let tail_cluster_id = self.get_last_cluster_in_chain(head)?;
        debug!("Tail cluster: {}", tail_cluster_id);

        let free_cluster_id = self.find_free_cluster()?.ok_or(FreeClusterNotFound)?;
        debug!("Free cluster found: {}", free_cluster_id);
        let last_entry = self.get_last_cluster_entry();
        self.write_entry_in_vfat_table(free_cluster_id, last_entry)?;
        info!("Written cluster");

        let updated_entry = FatEntry::from_chain(free_cluster_id);
        self.write_entry_in_vfat_table(tail_cluster_id, updated_entry)?;
        info!("Updated the entry");
        Ok(free_cluster_id)
    }
    fn write_entry_in_vfat_table(
        &self,
        cluster_id: ClusterId,
        entry: FatEntry,
    ) -> error::Result<()> {
        fat_writer::set_fat_entry(
            cluster_id,
            self.sector_size,
            self.device.clone(),
            self.fat_start_sector,
            entry,
        )
    }

    pub fn get_last_cluster_in_chain(&self, starting: ClusterId) -> error::Result<ClusterId> {
        info!("Getting last cluster in the chain..");
        let mut next = starting;
        while fat_reader::next_cluster(
            next,
            self.sector_size,
            self.device.clone(),
            self.fat_start_sector,
        )?
        .is_some()
        {
            next = fat_reader::next_cluster(
                next,
                self.sector_size,
                self.device.clone(),
                self.fat_start_sector,
            )?
            .unwrap();
            info!("next cluster: {}", next);
        }
        Ok(next)
    }
    pub fn cluster_reader(&self, cluster_id: ClusterId) -> cluster_reader::ClusterReader {
        cluster_reader::ClusterReader::new(
            self.device.clone(),
            cluster_reader::cluster_to_sector(
                cluster_id,
                self.sectors_per_cluster,
                self.data_start_sector,
            ),
            self.sectors_per_cluster,
            self.sector_size,
        )
    }
    pub fn cluster_writer(
        &self,
        cluster_id: ClusterId,
        offset_sector_in_cluster: SectorId,
        offset_in_sector: usize,
    ) -> cluster_writer::ClusterWriter {
        cluster_writer::ClusterWriter::new_offset(
            self.device.clone(),
            cluster_reader::cluster_to_sector(
                cluster_id,
                self.sectors_per_cluster,
                self.data_start_sector,
            ),
            offset_sector_in_cluster,
            self.sectors_per_cluster,
            self.sector_size,
            offset_in_sector,
        )
    }
    pub fn cluster_chain_writer(
        &self,
        cluster_id: ClusterId,
    ) -> cluster_writer::ClusterChainWriter {
        cluster_writer::ClusterChainWriter::new(self.clone(), cluster_id, SectorId(0))
    }

    pub fn cluster_chain_reader(
        &self,
        cluster_id: ClusterId,
    ) -> cluster_reader::ClusterChainReader {
        cluster_reader::ClusterChainReader::new(
            self.device.clone(),
            self.sector_size,
            self.sectors_per_cluster,
            cluster_id,
            self.data_start_sector,
            self.fat_start_sector,
        )
    }

    /// This will delete all the cluster chain starting from cluster_id.
    pub fn delete_fat_cluster_chain(&self, cluster_id: ClusterId) -> error::Result<()> {
        crate::fat_writer::delete_cluster_chain(
            cluster_id,
            self.sector_size,
            self.device.clone(),
            self.fat_start_sector,
        )
    }
    pub fn write_cluster_content(
        &self,
        cluster_id: ClusterId,
        source_buffer: &[u8],
    ) -> error::Result<usize> {
        info!("Requested write of cluster: {}", cluster_id);
        let sector = self.cluster_to_sector(cluster_id);
        info!("Sector: {:?}", sector);
        let mut mutex = self.device.as_ref();
        let amount_written = mutex.lock(|device| device.write_sector(sector, source_buffer))?;
        info!("Written: {:?}", amount_written);
        Ok(amount_written)
    }
    /// p should start with `/`.
    /// TODO: test.
    /// Test with a path to a file, test with a path to root.
    pub fn get_path(&mut self, path: Path) -> core::result::Result<VfatEntry, Error> {
        info!("FS: requested path: {:?}", path);
        if path == Path::new("/") {
            return self.get_root().map(From::from);
        }
        let mut path_iter = path.as_parts();
        let mut current_entry = VfatEntry::from(self.get_root()?);
        path_iter.next();
        for sub_path in path_iter {
            info!("Visiting path: {}", sub_path);
            let directory = current_entry
                .into_directory()
                .ok_or_else(|| binrw::io::Error::from(binrw::io::ErrorKind::NotFound))?;
            let directory_iter = directory.iter();
            let matches: Option<VfatEntry> = directory_iter
                .filter(|entry| {
                    info!(
                        "Entry name: {}, looking for sub_path: {}",
                        entry.metadata().name(),
                        sub_path
                    );
                    entry.metadata().name() == sub_path
                })
                .last();
            current_entry = matches.ok_or_else(|| {
                info!("Matches for {} is empty: path not found!", sub_path);
                binrw::io::Error::from(binrw::io::ErrorKind::NotFound)
            })?;
        }
        Ok(current_entry)
    }

    pub fn path_exists(&mut self, path: Path) -> core::result::Result<bool, binrw::io::Error> {
        let entry = self.get_path(path);
        match entry {
            Err(error) if matches!(error.kind(), binrw::io::ErrorKind::NotFound) => Ok(false),
            Ok(_) => Ok(true),
            Err(err) => Err(err),
        }
    }
    pub fn get_root(&mut self) -> core::result::Result<VfatDirectory, binrw::io::Error> {
        const UNKNOWN_ENTRIES: usize = 1;
        const BUF_SIZE: usize = UNKNOWN_ENTRIES * mem::size_of::<UnknownDirectoryEntry>();
        let mut buf = [0; BUF_SIZE];
        let mut cluster_reader = self.cluster_reader(self.root_cluster);
        let _amount_read = cluster_reader.read(&mut buf);
        let unknown_entries: UnknownDirectoryEntry = unsafe { mem::transmute(buf) };
        debug!("Unknown entries: {:?}", unknown_entries);
        let volume_id = VfatDirectoryEntry::from(unknown_entries)
            .into_regular()
            .filter(|regular| regular.is_volume_id())
            .ok_or_else(|| {
                binrw::io::Error::new(binrw::io::ErrorKind::NotFound, "Volume id not found?!")
            })?;

        let metadata = VfatMetadata::new(
            volume_id.creation_time,
            volume_id.last_modification_time,
            "/",
            mem::size_of::<RegularDirectoryEntry>() as u32,
            Path::new("/"),
            self.root_cluster,
            Path::new(""),
            Attributes::new_directory(),
        );
        Ok(VfatDirectory::new(self.clone(), metadata))
    }
    fn get_canonical_name() -> &'static str {
        "VFAT/FAT32"
    }
}
