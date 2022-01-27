use crate::os_interface::{VfatDirectory, VfatFile, VfatMetadata};
use crate::VfatFS;

#[derive(Debug)]
enum EntryKind {
    File,
    Directory,
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
