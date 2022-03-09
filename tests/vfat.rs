use binrw::io::{SeekFrom, Write};
use chrono::{DateTime, Datelike, Local};
use std::fs::OpenOptions;
use std::sync::Arc;

use log::info;
use rand::Rng;
use serial_test::serial;

use block_devs::{ArrayBackedBlockDevice, FilebackedBlockDevice};
use vfat_rs::mbr::MasterBootRecord;
use vfat_rs::sector_id::SectorId;
use vfat_rs::{
    mbr, BlockDevice, CachedPartition, ClusterId, EntryType, NullLock, Path, RawFatEntry, VfatFS,
};

mod block_devs;
mod common;
/*
   Vfat's integration tests. Why the serial annotation? Because each test is creating a new instance
   of VFAT, so they are not synchronized underneath (something that should not happen in the kernel were
   one is supposed to have one instance per device). Because wrapping the VFAT instance into a mutex
   would end up to just have them running in serial, I prefered to just go ahead and use `serial_test` crate.
*/
fn init() -> (FilebackedBlockDevice, MasterBootRecord) {
    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::builder().is_test(true).try_init();
    let path = common::setup();
    let mut fs = FilebackedBlockDevice {
        image: OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap(),
    };
    let mut buf = [0; 512];
    // MBR is always located in sector 0 of the disk
    fs.read_sector(SectorId(0), &mut buf).unwrap();
    let master_boot_record = MasterBootRecord::from(buf);
    (fs, master_boot_record)
}

fn init_vfat() -> vfat_rs::Result<VfatFS> {
    let (dev, master_boot_record) = init();
    info!("start: {:#?}", master_boot_record);
    VfatFS::new(dev, master_boot_record.partitions[0].start_sector)
}

fn random_name(prefix: &str) -> (String, String) {
    let mut rng = rand::thread_rng();
    let random_suffix: u32 = rng.gen_range(1..999);
    let name = format!("{}-{}.txt", prefix, random_suffix);
    let path = format!("/{}", name);
    (name, path)
}

#[test]
#[serial]
fn test_read_bios_parameter_block() {
    let (mut dev, master_boot_record) = init();

    assert_eq!(
        master_boot_record.valid_bootsector_sign,
        mbr::VALID_BOOTSECTOR_SIGN
    );

    let partition = master_boot_record.get_vfat_partition(0).unwrap();
    let fullbpb = VfatFS::read_fullebpb(&mut dev, partition.start_sector).unwrap();
    assert_eq!(
        String::from_utf8_lossy(fullbpb.extended.volume_label_string.as_ref()).trim(),
        "IRISVOL".to_string()
    );
}

#[test]
#[serial]
fn test_read_file() -> vfat_rs::Result<()> {
    let mut vfat = init_vfat()?;
    let expected_content = "Hello, Iris OS!".to_string();
    let mut file = vfat.get_path("/hello.txt".into())?.into_file().unwrap();
    let mut buf = [0; 512];
    file.read(&mut buf)?;
    assert_eq!(
        String::from_utf8_lossy(&buf[..expected_content.len()]),
        expected_content
    );

    const LONG_FILE: &[u8] = b"From fairest creatures we desire increase,
That thereby beauty's rose might never die,
But as the riper should by time decrease,
His tender heir mught bear his memeory:
But thou, contracted to thine own bright eyes,
Feed'st thy light'st flame with self-substantial fuel,
Making a famine where abundance lies,
Thyself thy foe, to thy sweet self too cruel.
Thou that art now the world's fresh ornament
And only herald to the gaudy spring,
Within thine own bud buriest thy content
And, tender churl, makest waste in niggarding.
Pity the world, or else this glutton be,
To eat the world's due, by the grave and thee.
";

    let mut file = vfat
        .get_path("/a-big-file.txt".into())?
        .into_file()
        .unwrap();
    info!(
        "Big file found!, size: {}, file size: {}",
        LONG_FILE.len(),
        file.metadata().size()
    );

    let mut buf = [0; LONG_FILE.len()];
    file.read(&mut buf)?;
    assert_eq!(LONG_FILE, &buf);
    assert_eq!(LONG_FILE.len(), file.metadata().size());

    const FIRST_LINE: &[u8] = b"From fairest creatures we desire increase,";
    let mut buf = [0u8; FIRST_LINE.len()];
    file.seek(SeekFrom::Start(0))?;

    file.read(&mut buf)?;
    assert_eq!(FIRST_LINE, &buf);

    const LAST_LINE: &[u8] = b"To eat the world's due, by the grave and thee.\n";
    let mut buf = [0u8; LAST_LINE.len()];
    file.seek(SeekFrom::End(-(LAST_LINE.len() as i64)))?;
    info!("Position: {}", file.offset);
    file.read(&mut buf)?;
    assert_eq!(LAST_LINE, &buf);

    const SECOND_CHAR: &[u8] = b"r";
    const THIRD_CHAR: &[u8] = b"o";

    let mut buf = [0u8; 1];
    file.seek(SeekFrom::Start(1))?;
    file.read(&mut buf)?;
    assert_eq!(buf, SECOND_CHAR);
    file.seek(SeekFrom::Start(2))?;
    file.read(&mut buf)?;
    assert_eq!(buf, THIRD_CHAR);

    file.seek(SeekFrom::Start(0))?;
    // seek to a position < 0
    file.seek(SeekFrom::Current(-1)).unwrap_err();
    // Seek to 0:
    file.seek(SeekFrom::End(-(LONG_FILE.len() as i64)))?;
    // seek to -1:
    file.seek(SeekFrom::End(-(LONG_FILE.len() as i64 + 1 as i64)))
        .unwrap_err();

    Ok(())
}

#[test]
#[serial]
fn test_path() {
    init();
    let expected = "/folder/something";
    let path = Path::new(expected);
    let pathb = path.clone();
    let mut path2 = pathb.as_parts();
    info!("First: {:?}", path2.next());
    info!("sec: {:?}", path2.next());
    assert_eq!(path.as_parts().collect::<Vec<&str>>().join("/"), expected);
    let expected = "/";
    let path = Path::new(expected);
    info!(
        "PATH: {:?}",
        path.as_parts().collect::<Vec<&str>>().join("/")
    );
}

#[test]
#[serial]
fn test_get_path() -> vfat_rs::Result<()> {
    use vfat_rs::VfatMetadataTrait;

    let mut vfat = init_vfat()?;
    vfat.get_path("/not-found.txt".into()).unwrap_err();
    let file = vfat.get_path("/hello.txt".into()).unwrap();
    let local: DateTime<Local> = Local::now();
    assert_eq!(file.creation().year(), local.year() as u32);
    assert_eq!(file.creation().month(), local.month());
    assert_eq!(file.creation().day(), local.day());
    assert!(file.creation().hour() <= 23);
    assert!(file.creation().minute() <= 60);
    assert!(file.creation().second() <= 60);
    info!("Hello txt found!");
    assert!(vfat
        .get_path("/folder/some/deep/nested/folder/file".into())
        .is_ok());
    Ok(())
}
#[test]
#[serial]
#[ignore]
fn test_list_directory() -> vfat_rs::Result<()> {
    //TODO: test directory listing
    //unimplemented!()
    Ok(())
}

#[test]
#[serial]
fn test_get_root() -> vfat_rs::Result<()> {
    let mut vfat = init_vfat()?;
    let entry = vfat.get_root().unwrap();
    assert_eq!(entry.metadata.path(), entry.metadata.name());
    assert_eq!(entry.metadata.path(), "/");
    info!("Entry:{:?}", entry);
    Ok(())
}

#[test]
#[serial]
fn test_find_next_free() {
    init();
    let mut ret = Vec::new();
    const RAW_ENTRY_SIZE: usize = core::mem::size_of::<RawFatEntry>();
    // Reserved entry:
    ret.extend_from_slice(&[0x01; RAW_ENTRY_SIZE]);
    // Free entry:
    ret.extend_from_slice(&[0x00; RAW_ENTRY_SIZE]);

    // Complete the sector:
    ret.extend_from_slice(&[0x01; 512 - (RAW_ENTRY_SIZE * 2)]);

    let dev = ArrayBackedBlockDevice {
        arr: ret,
        read_iteration: 0,
    };
    let vfat = vfat_rs::VfatFS {
        device: Arc::new(NullLock::new(CachedPartition::new(dev))),
        fat_start_sector: SectorId(0),
        data_start_sector: SectorId(2),
        sectors_per_cluster: 1,
        sectors_per_fat: 1,
        root_cluster: ClusterId::new(0),
        eoc_marker: Default::default(),
        sector_size: 1,
    };
    assert_eq!(
        vfat.find_free_cluster().unwrap().unwrap(),
        ClusterId::new(1)
    );
}

#[test]
#[serial]
fn test_write_side_short() -> vfat_rs::Result<()> {
    test_file_write("fl")
}

#[test]
#[serial]
fn test_file_write_long() -> vfat_rs::Result<()> {
    test_file_write("a-very-long-file-name")
}

#[test]
#[serial]
fn test_file_creation() -> vfat_rs::Result<()> {
    let invalid_name = "hello+world";
    let used_name_path = "/hello_world";

    let mut vfat = init_vfat()?;
    let mut root = vfat.get_root()?;

    // 2. assert file does not exists
    vfat.path_exists(used_name_path.into())
        .expect("File already exists. Please delete it.");

    // 3. create file
    root.create(invalid_name.into(), EntryType::File)
        .expect("Cannote create file");

    vfat.path_exists(used_name_path.into()).unwrap();
    Ok(())
}

fn test_file_write(name: &str) -> vfat_rs::Result<()> {
    let (file_name, file_path) = random_name(name);
    let mut vfat = init_vfat()?;
    let mut root = vfat.get_root()?;

    // 2. assert file does not exists
    vfat.path_exists(file_path.as_str().into())
        .expect("File already exists. Please delete it.");

    // 3. create file
    let res = root
        .create(file_name.clone(), EntryType::File)
        .expect("Cannote create file");

    // 4. Write CONTENT to file
    const CONTENT: &[u8] = b"Hello, world! This is Vfat\n";
    let mut as_file = res.into_file().expect("Into file");
    as_file.write_all(CONTENT).expect("write all");

    let mut as_file = vfat
        .get_path(file_path.as_str().into())
        .unwrap()
        .into_file()
        .unwrap();

    println!("File's metadata: {:?}", as_file.metadata());
    assert_eq!(
        as_file.metadata().size(),
        CONTENT.len(),
        "File's metadata size is wrong."
    );

    // 5. Read CONTENT back
    as_file.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = [0; CONTENT.len()];
    as_file.read(&mut buf).expect("Read exact");
    println!("Read: {}", String::from_utf8_lossy(&buf));
    assert_eq!(buf, CONTENT, "simple write failed");

    as_file.write(CONTENT).expect("second write");
    // return to 0.
    as_file
        .seek(SeekFrom::End(-(CONTENT.len() as i64) * 2))
        .unwrap();
    let mut double_buf = [0u8; CONTENT.len() * 2];

    as_file.read(&mut double_buf).unwrap();
    info!("Read: {:?}", String::from_utf8_lossy(&double_buf));
    assert_eq!(CONTENT, &double_buf[..CONTENT.len()], "first half");
    assert_eq!(CONTENT, &double_buf[CONTENT.len()..], "second half");

    root.delete(file_name.to_string()).expect("delete file");
    // 6. assert file does not exists
    let _file = vfat.get_path(file_path.as_str().into()).unwrap_err();
    Ok(())
}

#[test]
#[serial]
fn test_create_directory_long() -> vfat_rs::Result<()> {
    test_create_directory("some-uncommonly-long-folder-name")
}

#[test]
#[serial]
fn test_create_directory_short() -> vfat_rs::Result<()> {
    test_create_directory("fld")
}

fn test_create_directory(prefix: &str) -> vfat_rs::Result<()> {
    let (dir_name, dir_path) = random_name(prefix);
    let mut vfat = init_vfat()?;
    let mut root = vfat.get_root()?;

    let err = format!("Directory '{}' already exists. Please delete it.", dir_path);

    // 2. assert file does not exists
    let _file = vfat
        .get_path(dir_path.as_str().into())
        .expect_err(err.as_str());

    // 3. create directory
    let res = root.create(dir_name.clone(), EntryType::Directory)?;

    let sub_dir = "prova";
    res.into_directory()
        .unwrap()
        .create(sub_dir.to_string(), EntryType::Directory)?;
    let full_path = format!("/{}/{}", dir_name, sub_dir);
    vfat.get_path(Path::new(full_path))?;

    // Cleanup:
    vfat.get_path(Path::new(dir_path))?
        .into_directory_unchecked()
        .delete(sub_dir.to_string())?;
    vfat.get_root()?.delete(dir_name.to_string())?;
    Ok(())
}

#[test]
#[serial]
fn test_delete_folder_non_empty() -> vfat_rs::Result<()> {
    let (folder_name, _folder_path) = random_name("delfld");
    let mut vfat = init_vfat()?;
    let mut root = vfat.get_root()?;
    let mut folder = root
        .create(folder_name.clone(), EntryType::Directory)?
        .into_directory()
        .unwrap();
    let (subfolder_name, _subfolder_path) = random_name("subfld");
    folder.create(subfolder_name.clone(), EntryType::Directory)?;
    // cannot delete folder with some content:
    root.delete(folder_name.to_string()).unwrap_err();

    // deleting subcontent first should allow delete to succeed.
    folder.delete(subfolder_name.clone())?;
    root.delete(folder_name.to_string())?;

    Ok(())
}

#[test]
#[ignore]
#[serial]
fn test_stress() -> vfat_rs::Result<()> {
    // TODO: stress file creation
    Ok(())
}
