use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};

use binrw::io::{SeekFrom, Write};
use log::debug;

use vfat_rs::mbr::MasterBootRecord;
use vfat_rs::{BlockDevice, SectorId, VfatFS, VfatMetadataTrait};

fn main() {
    // to enable logging:
    // use env_logger::Env;
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut fbd = FilebackedBlockDevice {
        image: OpenOptions::new()
            .read(true)
            .write(true)
            .open("/tmp/irisos_fat32/fat32.fs")
            .unwrap(),
    };

    let mut buf = [0; 512];
    fbd.read_sector(SectorId(0), &mut buf).unwrap();

    // MBR is always located in sector 0 of the disk
    let master_boot_record = MasterBootRecord::from(buf);
    let mut vfat_fs = VfatFS::new(fbd, master_boot_record.partitions[0].start_sector).unwrap();
    let mut root = vfat_fs.get_root().unwrap();
    let contents = root.contents().unwrap();
    println!(
        "Content: {:?}",
        contents
            .into_iter()
            .map(|f| f.name().to_string())
            .collect::<Vec<_>>()
    );
    let mut file = root.create_file("my-file".to_string()).unwrap();
    let contents = root.contents().unwrap();
    println!(
        "Content: {:?}",
        contents
            .into_iter()
            .map(|f| f.name().to_string())
            .collect::<Vec<_>>()
    );
    let to_write = b"Hello, world!";
    file.write(to_write).unwrap();
    let mut buf = [0u8; 13];
    // TODO: is seek usually needed?
    file.seek(SeekFrom::Start(0)).unwrap();

    file.read(&mut buf).unwrap();
    println!("The file contains: '{}'", String::from_utf8_lossy(&buf));
    root.delete("my-file".to_string()).unwrap();
    println!("File was deleted!");
    let contents = root.contents().unwrap();
    println!(
        "Content: {:?}",
        contents
            .into_iter()
            .map(|f| f.name().to_string())
            .collect::<Vec<_>>()
    );
}

pub struct FilebackedBlockDevice {
    pub image: File,
}

impl BlockDevice for FilebackedBlockDevice {
    fn sector_size(&self) -> usize {
        512
    }

    fn read_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        mut buf: &mut [u8],
    ) -> vfat_rs::Result<usize> {
        let max_read = min(buf.len(), self.sector_size());
        let mut temp_buf = vec![0; max_read];
        let final_destination = sector.0 as u64 * self.sector_size() as u64 + offset as u64;
        debug!(
            "Sector: {}, offset: {}, finaldest: {}",
            sector.0 as u64 * self.sector_size() as u64,
            offset,
            final_destination
        );
        self.image
            .seek(std::io::SeekFrom::Start(final_destination))
            .expect("Impossible to seek to the sector");

        self.image
            .read_exact(temp_buf.as_mut_slice())
            .expect("Impossible to read from image");
        debug!("done reading read_sector_offset...");
        buf.write(temp_buf.as_mut_slice()).map_err(Into::into)
    }

    fn write_sector_offset(
        &mut self,
        sector: SectorId,
        offset: usize,
        buf: &[u8],
    ) -> vfat_rs::Result<usize> {
        use std::io::Write;
        let final_destination = sector.0 as u64 * self.sector_size() as u64 + offset as u64;
        debug!(
            "Seeking to : sector: {}, sector_size: {}, offset: {}, final destination: {} ",
            sector,
            self.sector_size(),
            offset,
            final_destination
        );
        self.image
            .seek(std::io::SeekFrom::Start(final_destination))
            .expect("Error seek");
        debug!("Writing the buffer to the image..");
        self.image.write_all(buf).expect("Write sector");
        debug!("Written: {}", buf.len());
        self.image.flush().unwrap();
        Ok(buf.len())
    }

    fn get_canonical_name() -> &'static str
    where
        Self: Sized,
    {
        "FileBasedBlockDevice"
    }
}
