use std::path::PathBuf;
use std::process::Command;
use std::{fs, io};

pub fn setup() -> PathBuf {
    println!("Running setup script...");
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests/setup.sh");
    let fs_path = "/tmp/irisos_fat32/fat32.fs";
    let hello = Command::new("bash")
        .arg(d.display().to_string().as_str())
        .output()
        .expect("failed to execute setup script.");
    println!("Setup script output: {:?}", hello);
    fs_path.into()
}

pub fn purge_fs() -> io::Result<()> {
    fs::remove_file("/tmp/irisos_fat32/fat32.fs").unwrap();
    fs::remove_file("/tmp/irisos_fat32/a-big-file.txt").unwrap();
    fs::remove_file("/tmp/irisos_fat32/a-very-long-file-name-entry.txt").unwrap();
    fs::remove_file("/tmp/irisos_fat32/hello.txt").unwrap();

    fs::remove_file("/tmp/irisos_fat32/folder/some/deep/nested/folder/file").unwrap();
    fs::remove_dir("/tmp/irisos_fat32/folder/some/deep/nested/folder/").unwrap();
    fs::remove_dir("/tmp/irisos_fat32/folder/some/deep/nested/").unwrap();
    fs::remove_dir("/tmp/irisos_fat32/folder/some/deep/").unwrap();
    fs::remove_dir("/tmp/irisos_fat32/folder/some/").unwrap();
    fs::remove_dir("/tmp/irisos_fat32/folder/").unwrap();

    if PathBuf::from("/tmp/irisos_fat32/MyFoLdEr").exists() {
        fs::remove_dir("/tmp/irisos_fat32/MyFoLdEr").unwrap();
    }

    fs::remove_dir("/tmp/irisos_fat32/").unwrap();
    Ok(())
}
