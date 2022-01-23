use binrw::io::Write;
use std::cmp::min;
use std::fs::File;
use std::io::SeekFrom;
use std::io::{Read, Seek};
use vfat_rs::sector_id::SectorId;
use vfat_rs::BlockDevice;

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
        println!("Max read: {}", max_read);
        let final_destination = sector.0 as u64 * self.sector_size() as u64 + offset as u64;
        println!(
            "Sector: {}, offset: {}, finaldest: {}",
            sector.0 as u64 * self.sector_size() as u64,
            offset,
            final_destination
        );
        self.image
            .seek(SeekFrom::Start(final_destination))
            .expect("Impossible to seek to the sector");

        self.image
            .read(temp_buf.as_mut_slice())
            .expect("Impossible to read from image");
        println!("done reading...");
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
        println!(
            "Seeking to : sector: {}, sector_size: {}, offset: {}, final destination: {} ",
            sector,
            self.sector_size(),
            offset,
            final_destination
        );
        self.image
            .seek(SeekFrom::Start(final_destination))
            .expect("Error seek");
        println!("Writing the buffer to the image..");
        self.image.write_all(buf).expect("Write sector");
        println!("Written: {}", buf.len());
        Ok(buf.len())
    }

    fn get_canonical_name() -> &'static str
    where
        Self: Sized,
    {
        "FileBasedBlockDevice"
    }
}
