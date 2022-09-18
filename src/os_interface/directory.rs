use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::{IntoIter, Vec};
use core::mem;

use log::{debug, error, info};
use snafu::ensure;

use crate::cluster::cluster_reader::ClusterChainReader;
use crate::cluster::cluster_writer::ClusterChainWriter;
use crate::os_interface::directory_entry::{
    unknown_entry_convert_to_bytes_2, Attributes, EntryId, RegularDirectoryEntry,
    UnknownDirectoryEntry, VfatDirectoryEntry,
};
use crate::os_interface::timestamp::VfatTimestamp;
use crate::os_interface::{VfatEntry, VfatMetadata};
use crate::{error, Path};
use crate::{ClusterId, SectorId, VfatFS, VfatMetadataTrait};

// TODO: this assumes sector size
const SECTOR_SIZE: usize = 512;
const ENTRIES_AMOUNT: usize = SECTOR_SIZE / mem::size_of::<UnknownDirectoryEntry>();
const BUF_SIZE: usize = mem::size_of::<UnknownDirectoryEntry>() * ENTRIES_AMOUNT;

pub fn unknown_entry_convert_from_bytes_entries(
    entries: [u8; BUF_SIZE],
) -> [UnknownDirectoryEntry; ENTRIES_AMOUNT] {
    unsafe { mem::transmute(entries) }
}

pub enum EntryType {
    File,
    Directory,
    // Link
}

/// This is the public interface to the directory concept.
#[derive(Debug)]
pub struct VfatDirectory {
    pub(crate) vfat_filesystem: VfatFS,
    pub metadata: VfatMetadata,
}

impl VfatDirectory {
    pub fn new(vfat_filesystem: VfatFS, metadata: VfatMetadata) -> Self {
        Self {
            vfat_filesystem,
            metadata,
        }
    }

    fn create_metadata_for_new_entry(
        &mut self,
        entry_name: &str,
        entry_type: &EntryType,
    ) -> error::Result<VfatMetadata> {
        let path = Path::new(format!("{}{}", self.metadata.path(), entry_name));
        let attributes = Self::attributes_from_entry(entry_type);
        let cluster_id = match entry_type {
            EntryType::File => ClusterId::new(0),
            EntryType::Directory => self.vfat_filesystem.allocate_cluster_new_entry()?,
        };
        info!("Going to use as cluster id: {}", cluster_id);
        let size = 0;
        let metadata = VfatMetadata::new(
            VfatTimestamp::new(0),
            VfatTimestamp::new(0),
            entry_name,
            size,
            path,
            cluster_id,
            self.metadata.path().clone(),
            attributes,
        );
        Ok(metadata)
    }
}

impl VfatDirectory {
    pub(crate) fn iter(&self) -> IntoIter<VfatEntry> {
        self.contents().unwrap().into_iter()
    }
    fn contains(&self, name: &str) -> error::Result<bool> {
        for entry in self.contents()? {
            if entry.name() == name {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn create(&mut self, name: String, entry_type: EntryType) -> error::Result<VfatEntry> {
        if self.contains(&name)? {
            return Err(error::VfatRsError::NameAlreadyInUse { target: name });
        }

        //1. Create metadata:
        let metadata = self.create_metadata_for_new_entry(name.as_str(), &entry_type)?;

        // 2. Based on the name, create one or more LFN and the Regular entry.
        let entries: Vec<UnknownDirectoryEntry> = VfatDirectoryEntry::new_vfat_entry(
            name.as_str(),
            metadata.cluster,
            Self::attributes_from_entry(&entry_type),
        );
        let spots_needed = entries.len();
        let found = self.find_empty_spots_with_cluster(spots_needed)?;
        let (found_spot_start_index, cluster_id) = match found {
            Some(spot) => spot,
            None => {
                debug!(
                    "No free spot for the new entry found in the currently available cluster. \
        Going to allocate a new one and trying again."
                );
                // Keep storing LFNs and entries.
                // Apparently, we did not found any free spot in the cluster.
                let cluster_id = self
                    .vfat_filesystem
                    .allocate_cluster_to_chain(self.metadata.cluster)?;
                info!(
                    "Cluster id: {} was successfully allocated. Going to assign entries now.",
                    cluster_id
                );
                (0, cluster_id)
            }
        };

        info!(
            "Going to use as metadata: {:?}. self metadatapath= '{}', selfmetadata name = '{}'. My attributes: {:?}, cluster: {:?}",
            metadata,
            self.metadata.path(),
            self.metadata.name(),
            self.metadata.attributes,
            self.metadata.cluster
        );
        info!(
            "Found spot: {}, Going to append entries: {:?}",
            found_spot_start_index, entries
        );
        let spot_memory_offset = found_spot_start_index * mem::size_of::<UnknownDirectoryEntry>();
        let offset_in_sector = spot_memory_offset % self.vfat_filesystem.sector_size;

        let sector_offset = (spot_memory_offset / self.vfat_filesystem.sector_size) as u32;

        let mut ccw = ClusterChainWriter::new_w_offset(
            self.vfat_filesystem.clone(),
            cluster_id,
            SectorId(sector_offset),
            offset_in_sector,
        );
        info!(
            "found spot: {}, offset_in_sector = {}, start_sector = {}, cluster: {}",
            found_spot_start_index, offset_in_sector, sector_offset, cluster_id
        );
        for unknown_entry in entries.into_iter() {
            let entry: [u8; mem::size_of::<UnknownDirectoryEntry>()] = unknown_entry.into();
            ccw.write(&entry)?;
        }

        if let EntryType::Directory = entry_type {
            let entries =
                VfatDirectoryEntry::create_pseudo_dir_entries(metadata.cluster, ClusterId::new(0));
            let mut cw = self.vfat_filesystem.cluster_chain_writer(metadata.cluster);
            let buf = unknown_entry_convert_to_bytes_2(entries);
            cw.write(&buf)?;
        }

        Ok(match entry_type {
            EntryType::Directory => {
                VfatEntry::new_directory(metadata, self.vfat_filesystem.clone())
            }
            EntryType::File => VfatEntry::new_file(metadata, self.vfat_filesystem.clone()),
        })
    }

    /// Returns an entry from inside this directory.
    fn get_entry(&mut self, target_filename: String) -> error::Result<VfatEntry> {
        self.contents()?
            .into_iter()
            .find(|name| {
                debug!(
                    "Checking name: {} == {}",
                    name.metadata.name(),
                    target_filename
                );
                name.metadata.name() == target_filename
            })
            .ok_or(error::VfatRsError::FileNotFound {
                target: target_filename,
            })
    }

    //TOOD: test pseudo dir deletion.
    pub fn delete(&mut self, target_name: String) -> error::Result<()> {
        info!("Starting delete routine for entry: '{}'. ", target_name);
        info!("Directory contents: {:?}", self.contents()?);

        const PSEUDO_CURRENT_FOLDER: &str = ".";
        const PSEUDO_PARENT_FOLDER: &str = "..";
        const PSEUDO_FOLDERS: &[&str; 2] = &[PSEUDO_PARENT_FOLDER, PSEUDO_CURRENT_FOLDER];

        ensure!(
            !PSEUDO_FOLDERS.contains(&target_name.as_str()),
            error::CannotDeletePseudoDirSnafu {
                target: target_name,
            }
        );

        let target_entry = self.get_entry(target_name)?;

        info!("Found target entry: {:?}", target_entry);
        self.delete_entry(target_entry)
    }

    fn contents(&self) -> error::Result<Vec<VfatEntry>> {
        info!("Directory contents, cluster: {:?}", self.metadata.cluster);

        let mut buf = [0; BUF_SIZE];
        let mut contents = Vec::new();
        let filter_invalid =
            |entry: &VfatDirectoryEntry| !matches!(*entry, VfatDirectoryEntry::EndOfEntries(_));
        let mut cluster_chain_reader = self
            .vfat_filesystem
            .cluster_chain_reader(self.metadata.cluster);

        let mut entries = Vec::new();
        while cluster_chain_reader.read(&mut buf)? > 0 {
            let unknown_entries: [UnknownDirectoryEntry; ENTRIES_AMOUNT] =
                unknown_entry_convert_from_bytes_entries(buf);
            debug!("Unknown entries: {:?}", unknown_entries);
            #[cfg(debug_assertions)]
            unknown_entries
                .iter()
                .map(VfatDirectoryEntry::from)
                //.take_while(filter_invalid)
                .for_each(|entry| info!("unknown entry to vfat directory entry: {:?}", entry));

            unknown_entries
                .iter()
                .map(VfatDirectoryEntry::from)
                .filter(filter_invalid)
                .for_each(|entry| {
                    entries.push(entry);
                })
        }
        let mut lfn_buff: Vec<(u8, String)> = Vec::new();
        for dir_entry in entries {
            info!("Found entry: {:?}", dir_entry);
            match dir_entry {
                VfatDirectoryEntry::LongFileName(lfn) => {
                    lfn_buff.push((lfn.sequence_number.get_position(), lfn.collect_name()))
                }
                VfatDirectoryEntry::Deleted(_) => {
                    lfn_buff.clear();
                }
                VfatDirectoryEntry::Regular(regular) => {
                    let name = if !lfn_buff.is_empty() {
                        lfn_buff.sort();
                        let ret = lfn_buff
                            .into_iter()
                            .map(|(_, name)| name)
                            .collect::<Vec<String>>()
                            .join("");
                        lfn_buff = Vec::new();
                        ret
                    } else {
                        regular.full_name()
                    };

                    let metadata = VfatMetadata::new(
                        regular.creation_time,
                        regular.last_modification_time,
                        name.clone(),
                        regular.file_size,
                        Path::new(format!(
                            "{}{name}{}",
                            self.metadata.path(),
                            if regular.is_dir() { "/" } else { "" }
                        )),
                        regular.cluster(),
                        self.metadata.path().clone(),
                        regular.attributes,
                    );
                    info!(
                        "dir_entry: name: {name:?} - ClusterID: {}, file size: {}",
                        metadata.cluster,
                        metadata.size,
                        name = name.trim_end(),
                    );
                    info!("Metadata: {:?}", metadata);

                    let new_fn = if regular.is_dir() {
                        VfatEntry::new_directory
                    } else {
                        VfatEntry::new_file
                    };

                    contents.push(new_fn(metadata, self.vfat_filesystem.clone()));
                    //info!("New contents: {:?}", contents);
                }
                other => info!("Found other: {:?}", other),
            }
        }
        Ok(contents)
    }

    pub(crate) fn update_entry(&mut self, metadata: VfatMetadata) -> error::Result<()> {
        let target_name = metadata.name().to_string();
        info!("Running update entry on target name: {}", target_name);
        let regular: RegularDirectoryEntry = metadata.into();
        self.update_entry_inner(target_name, regular.into())
    }
}

impl VfatDirectory {
    fn cluster_chain_reader(&self) -> ClusterChainReader {
        self.vfat_filesystem
            .cluster_chain_reader(self.metadata.cluster)
    }

    // TODO: Currently this doesn't support renaming file, just updating metadatas...
    fn update_entry_inner(
        &mut self,
        target_name: String,
        new_entry: UnknownDirectoryEntry,
    ) -> error::Result<()> {
        info!("Running update entry routine...");
        let mut buf = [0; BUF_SIZE];

        let mut lfn_buff: Vec<(u8, String)> = Vec::new();

        let mut cluster_chain_reader = self.cluster_chain_reader();
        loop {
            if 0 == cluster_chain_reader.read(&mut buf)? {
                info!("Cluster chain reader is over.");
                break;
            }
            let unknown_entries: [UnknownDirectoryEntry; ENTRIES_AMOUNT] =
                unknown_entry_convert_from_bytes_entries(buf);

            for (index, dir_entry) in unknown_entries
                .iter()
                .map(Clone::clone)
                .map(VfatDirectoryEntry::from)
                .take_while(|entry| !matches!(*entry, VfatDirectoryEntry::EndOfEntries(_)))
                .enumerate()
            {
                match dir_entry {
                    VfatDirectoryEntry::LongFileName(lfn) => {
                        lfn_buff.push((lfn.sequence_number.get_position(), lfn.collect_name()))
                    }
                    VfatDirectoryEntry::Deleted(_) => lfn_buff.clear(),
                    VfatDirectoryEntry::Regular(regular) => {
                        let name = if !lfn_buff.is_empty() {
                            lfn_buff.sort();
                            let ret = lfn_buff
                                .into_iter()
                                .map(|(_, name)| name)
                                .collect::<Vec<String>>()
                                .join("");
                            lfn_buff = Vec::new();
                            ret
                        } else {
                            regular.full_name()
                        };
                        if name == target_name {
                            info!("Directory entry update: Found '{}'.", name);
                            self.update_entry_by_index(
                                new_entry,
                                index,
                                cluster_chain_reader.last_cluster_read,
                            )?;
                            return Ok(());
                        }
                    }
                    _ => {}
                };
            }
        }
        error!("Directory update entry {}: file not found!!", target_name);
        Err(error::VfatRsError::FileNotFound {
            target: target_name,
        })
    }
    fn delete_entry(&mut self, entry: VfatEntry) -> error::Result<()> {
        const SPECIAL_CURRENT_UPPER_DIRECTORY: usize = 2;
        let entry = if entry.is_dir() {
            let directory = entry.into_directory_unchecked();
            if directory.contents()?.len() > SPECIAL_CURRENT_UPPER_DIRECTORY {
                return Err(error::VfatRsError::NonEmptyDirectory {
                    target: directory.metadata.name().to_string(),
                });
            }
            info!("Target entry is a directory with no contents. It's safe to delete.");
            directory.into()
        } else {
            entry
        };

        info!(
            "Deleting entry's associated clusters starting at {:?}",
            entry.metadata.cluster
        );
        self.vfat_filesystem
            .delete_fat_cluster_chain(entry.metadata.cluster)?;
        let target_name = entry.metadata().name().to_string();
        // Directory Entry change to DeleteEntry
        // 2. Set VfatDirectoryEntry to Deleted.
        let dir_entry: RegularDirectoryEntry = entry.metadata.into();
        let mut dir_entry: UnknownDirectoryEntry = dir_entry.into();

        dir_entry.set_id(EntryId::Deleted);
        self.update_entry_inner(target_name, dir_entry)
    }

    pub(crate) fn update_entry_by_index(
        &self,
        entry: UnknownDirectoryEntry,
        index: usize,
        cluster: ClusterId,
    ) -> error::Result<()> {
        let entries_per_sector =
            self.vfat_filesystem.sector_size / mem::size_of::<UnknownDirectoryEntry>();
        let containing_sector = (index as f64 / entries_per_sector as f64).floor() as u32;
        let offset_in_sector = (index % entries_per_sector)
            .checked_mul(mem::size_of::<UnknownDirectoryEntry>())
            .unwrap();

        let buf: [u8; mem::size_of::<UnknownDirectoryEntry>()] = entry.into();

        debug!("Update entry by index, going to update entry index: {}, in sectorId: {}, in cluster: {}, with offset_in_sector: {}",
        index, containing_sector, self.metadata.cluster, offset_in_sector);

        let mut ccw = ClusterChainWriter::new_w_offset(
            self.vfat_filesystem.clone(),
            cluster,
            SectorId(containing_sector),
            offset_in_sector,
        );
        ccw.write(&buf)?;
        Ok(())
    }
    /// Searches for `spots_needed` in all the clusters allocated to this directory
    /// Will return None if not enough spots were found.
    fn find_empty_spots_with_cluster(
        &self,
        spots_needed: usize,
    ) -> error::Result<Option<(usize, ClusterId)>> {
        assert!(spots_needed > 0);
        info!(
            "Going to look for a spot, starting from: {}",
            self.metadata.cluster
        );
        let mut cluster_chain_reader = self
            .vfat_filesystem
            .cluster_chain_reader(self.metadata.cluster);
        let mut buff = [0u8; BUF_SIZE];
        let mut spots_found = 0;

        let mut start_cluster = None;
        let mut start_index = 0;

        while cluster_chain_reader.read(&mut buff)? > 0 {
            let unknown_entries: [UnknownDirectoryEntry; ENTRIES_AMOUNT] =
                unknown_entry_convert_from_bytes_entries(buff);
            for (index, entry) in unknown_entries.iter().enumerate() {
                if entry.last_entry() {
                    if start_cluster.is_none() {
                        start_cluster = Some(cluster_chain_reader.last_cluster_read);
                        start_index = index;
                        info!(
                            "First empty spot found! {:?}, {}",
                            start_cluster, start_index
                        );
                    }
                    spots_found += 1;
                } else {
                    spots_found = 0;
                    start_index = 0;
                    start_cluster = None;
                }
                if spots_needed == spots_found {
                    debug!(
                        "Found empty spot: {:?}, cluster: {:?}",
                        start_index,
                        start_cluster.unwrap()
                    );
                    return Ok(Some((start_index, start_cluster.unwrap())));
                }
            }
            buff = [0u8; BUF_SIZE];
        }
        Ok(None)
    }

    fn attributes_from_entry(entry: &EntryType) -> Attributes {
        match entry {
            EntryType::Directory => Attributes::new_directory(),
            EntryType::File => Attributes(0),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use crate::os_interface::directory_entry::EntryId;

    #[test]
    fn valid_entry_id() {
        let id: u8 = 0x10;
        assert!(matches!(EntryId::from(id), EntryId::Valid(_)));
        //id
    }
}
