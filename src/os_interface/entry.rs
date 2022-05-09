use crate::os_interface::{VfatDirectory, VfatFile, VfatMetadata};
use crate::timestamp::VfatTimestamp;
use crate::{Result, VfatFS};

#[derive(Debug)]
enum EntryKind {
    File,
    Directory,
}
pub trait VfatMetadataTrait {
    fn metadata(&self) -> &VfatMetadata;
    fn name(&self) -> &str {
        self.metadata().name()
    }
    fn creation(&self) -> VfatTimestamp {
        self.metadata().creation().unwrap()
    }
}

impl VfatMetadataTrait for VfatFile {
    fn metadata(&self) -> &VfatMetadata {
        &self.metadata
    }
}
impl VfatMetadataTrait for VfatDirectory {
    fn metadata(&self) -> &VfatMetadata {
        &self.metadata
    }
}
impl VfatMetadataTrait for VfatEntry {
    fn metadata(&self) -> &VfatMetadata {
        &self.metadata
    }
}

#[derive(Debug)]
pub struct VfatEntry {
    kind: EntryKind,
    pub metadata: VfatMetadata,
    vfat_filesystem: VfatFS,
}
impl VfatEntry {
    pub fn new_file(metadata: VfatMetadata, vfat_filesystem: VfatFS) -> Self {
        Self {
            kind: EntryKind::File,
            metadata,
            vfat_filesystem,
        }
    }
    pub fn new_directory(metadata: VfatMetadata, vfat_filesystem: VfatFS) -> Self {
        Self {
            kind: EntryKind::Directory,
            metadata,
            vfat_filesystem,
        }
    }
}

impl VfatEntry {
    pub(crate) fn metadata(&self) -> &VfatMetadata {
        &self.metadata
    }

    pub(crate) fn is_dir(&self) -> bool {
        matches!(&self.kind, EntryKind::Directory)
    }

    pub fn into_directory(self) -> Option<VfatDirectory> {
        self.is_dir()
            .then(|| VfatDirectory::new(self.vfat_filesystem, self.metadata))
    }
    pub fn into_directory_unchecked(self) -> VfatDirectory {
        VfatDirectory::new(self.vfat_filesystem, self.metadata)
    }
    pub fn into_directory_or_not_found(self) -> Result<VfatDirectory> {
        if self.is_dir() {
            Ok(self.into_directory_unchecked())
        } else {
            Err(crate::error::VfatRsError::EntryNotFound {
                target: self.metadata.name().into(),
            })
        }
    }
    fn is_file(&self) -> bool {
        !self.is_dir()
    }
    pub fn into_file(self) -> Option<VfatFile> {
        self.is_file()
            .then(|| VfatFile::new(self.vfat_filesystem, self.metadata))
    }
}

impl From<VfatFile> for VfatEntry {
    fn from(file: VfatFile) -> Self {
        VfatEntry::new_file(file.metadata, file.vfat_filesystem)
    }
}
impl From<VfatDirectory> for VfatEntry {
    fn from(directory: VfatDirectory) -> Self {
        VfatEntry::new_directory(directory.metadata, directory.vfat_filesystem)
    }
}
