use crate::os_interface::directory_entry::Attributes;
use crate::os_interface::Path;
use crate::timestamp::VfatTimestamp;
use crate::ClusterId;
use alloc::string::String;

#[derive(Debug, Clone)]
pub struct VfatMetadata {
    creation: VfatTimestamp,
    last_update: VfatTimestamp,
    //last_access: VfatTimestamp,
    name: String,
    /// Size of this file in bytes. For directories, it should be the sum of the sizes
    /// occupied by the metadatas of the contained files.
    pub(crate) size: u32,
    /// The path to this file - it does include the file name.
    path: Path,
    /// empty files with size 0 should have first cluster 0.
    pub(crate) cluster: ClusterId,
    /// The path to this file - it doesn't include the file name.
    parent: Path,
    pub(crate) attributes: Attributes,
}

impl VfatMetadata {
    pub fn new<S: AsRef<str>>(
        creation: VfatTimestamp,
        last_update: VfatTimestamp,
        //last_access: VfatTimestamp,
        name: S,
        size: u32,
        path: Path,
        cluster: ClusterId,
        parent: Path,
        attributes: Attributes,
    ) -> Self {
        Self {
            creation,
            last_update,
            //last_access,
            name: String::from(name.as_ref()),
            size,
            path,
            cluster,
            parent,
            attributes,
        }
    }
}
impl VfatMetadata {
    pub fn size(&self) -> usize {
        self.size as usize
    }

    /*
    fn last_access(&self) -> Option<VfatTimestamp> {
        //Some(self.last_access)
        None
    }
    */

    pub(crate) fn last_update(&self) -> Option<VfatTimestamp> {
        Some(self.last_update)
    }
    // TODO: why are these optional?
    pub(crate) fn creation(&self) -> Option<VfatTimestamp> {
        Some(self.creation)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn parent(&self) -> &Path {
        &self.parent
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}
