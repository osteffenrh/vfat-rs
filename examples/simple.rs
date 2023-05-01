use std::fs::OpenOptions;
use std::io::SeekFrom;

use vfat_rs::mbr::MasterBootRecord;
use vfat_rs::{BlockDevice, FilebackedBlockDevice, SectorId, VfatFS, VfatMetadataTrait};

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
