use std::fs::File;
use std::io::Read;
use vfat_rs::mbr;

mod common;

#[test]
fn test_mbr_reader() {
    common::setup();
    let _fs_path = common::setup();
    let fs_path = "/tmp/irisos_fat32/fat32.fs";
    let mut f = File::open(fs_path).expect("File not found");
    let mut buf = [0u8; 512];
    f.read_exact(&mut buf).expect("Cannot read!");

    let mbr = mbr::MasterBootRecord::from(buf);
    assert_eq!(mbr.valid_bootsector_sign, mbr::VALID_BOOTSECTOR_SIGN);
    let first_part = &mbr.partitions[0];
    assert_eq!(first_part.partition_type, 0xC);
    assert_eq!(
        first_part.bootable_indicator_flag,
        mbr::BOOTABLE_PARTITION_FLAG
    );
    common::purge_fs().unwrap();
}
