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
    fs::remove_dir("/tmp/irisos_fat32/").unwrap();
    Ok(())
}
