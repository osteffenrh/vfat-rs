use core::cmp;

use binrw::io::{Read, Seek, SeekFrom, Write};
use log::{debug, info};

use crate::os_interface::VfatMetadata;
use crate::{error, ClusterId, MutexTrait, VfatFS};

/// A File representation in a VfatFilesystem.
//#[derive(Clone)]
pub struct VfatFile {
    pub(crate) vfat_filesystem: VfatFS,
    pub(crate) metadata: VfatMetadata,
    // Current Seek position
    pub offset: usize,
}

impl VfatFile {
    pub fn new(vfat_filesystem: VfatFS, metadata: VfatMetadata) -> Self {
        VfatFile {
            vfat_filesystem,
            metadata,
            offset: 0,
        }
    }
    pub fn metadata(&self) -> &VfatMetadata {
        &self.metadata
    }

    fn update_file_size(&mut self, amount_written: usize) -> error::Result<()> {
        if self.offset + amount_written <= self.metadata.size as usize {
            return Ok(());
        }
        info!(
            "Offset: {}, written: {}, old size: {}",
            self.offset, amount_written, self.metadata.size
        );
        self.metadata.size = (self.offset + amount_written) as u32;
        info!("New file size: {}", self.metadata.size);
        info!(
            "I'm going to update file size on the fs... Parent path: {:?}",
            self.metadata.parent()
        );
        self.update_metadata()
    }
    fn update_metadata(&mut self) -> error::Result<()> {
        debug!("Going to update metadata on disk...");
        self.vfat_filesystem
            .get_path(self.metadata.parent().clone())?
            .into_directory()
            .unwrap()
            .update_entry(self.metadata.clone())
    }
}
impl VfatFile {
    fn sync(&mut self) -> error::Result<()> {
        unimplemented!()
    }
}

impl Write for VfatFile {
    fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, binrw::io::Error> {
        if buf.len() == 0 {
            return Ok(0);
        }
        debug!("Requested write on file.");
        if self.metadata.cluster == ClusterId(0) {
            debug!("File's cluster is none.");
            self.metadata.cluster = self.vfat_filesystem.allocate_cluster_new_entry()?;
            debug!("Allocated cluster to file: {}", self.metadata.cluster);
            self.update_metadata()?;
            debug!("Updated metadata");
        }
        let mut ccw = self
            .vfat_filesystem
            .cluster_chain_writer(self.metadata.cluster.clone());
        // TODO: FIXME.
        ccw.seek(self.offset)
            .map_err(|err| binrw::io::ErrorKind::Other)?;

        info!(
            "File: Write: Clusterid: {} amount to write: {}",
            self.metadata.cluster,
            buf.len()
        );
        let amount_written = ccw.write(buf)?;
        info!("File: Write: Amount written: {}", amount_written);

        self.update_file_size(amount_written)?;

        Ok(amount_written)
    }

    fn flush(&mut self) -> core::result::Result<(), binrw::io::Error> {
        let mut mutex = self.vfat_filesystem.device.as_ref();
        Ok(mutex.lock(|dev| dev.flush())?)
    }
}
impl Seek for VfatFile {
    fn seek(&mut self, pos: SeekFrom) -> core::result::Result<u64, binrw::io::Error> {
        match pos {
            SeekFrom::Start(val) => {
                self.offset = val as usize;
            }
            SeekFrom::End(val) => {
                if self.metadata.size as i64 + val < 0 {
                    // TODO: "Invalid argument - offset cannot be less then zero.",
                    return Err(binrw::io::Error::new(
                        binrw::io::ErrorKind::InvalidInput,
                        "Invalid argument - offset cannot be less then zero.",
                    ));
                }
                debug!(
                    "Seek from end, size: {}, movement: {}",
                    self.metadata.size, val
                );
                self.offset = (self.metadata.size as i64 + val) as usize;
            }
            SeekFrom::Current(val) => {
                if self.offset as i64 + val < 0 {
                    return Err(binrw::io::Error::new(
                        binrw::io::ErrorKind::InvalidInput,
                        "Invalid argument - offset cannot be less then zero.",
                    ));
                }
                self.offset = (self.offset as i64 + val) as usize
            }
        }
        Ok(self.offset as u64)
    }
}

/// The read will actually pull out data from the file
impl Read for VfatFile {
    fn read(&mut self, mut buf: &mut [u8]) -> core::result::Result<usize, binrw::io::Error> {
        // it should read at most the buf size or the missing file data.
        let amount_to_read = cmp::min(buf.len(), self.metadata.size().saturating_sub(self.offset));
        if amount_to_read == 0
            || self.metadata.cluster == ClusterId(0)
            || self.offset > self.metadata.size as usize
        {
            info!(
                "Amount to read: {}, cluster: {}, offset: {}, size: {}",
                amount_to_read, self.metadata.cluster, self.offset, self.metadata.size
            );
            return Ok(0);
        }
        let mut ccr = self
            .vfat_filesystem
            .cluster_chain_reader(self.metadata.cluster.clone());
        info!("Going to seek to:{}", self.offset);
        ccr.seek(self.offset)?;

        info!(
            "File: Clusterid: {} amount to read: {}, file size: {}",
            self.metadata.cluster, amount_to_read, self.metadata.size
        );
        buf = &mut buf[..amount_to_read];
        let amount_read = ccr.read(buf)?;
        self.offset += amount_read;
        Ok(amount_read)
    }
}
