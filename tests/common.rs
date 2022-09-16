use std::path::PathBuf;
use std::process::Command;
use std::{fs, io};

const FS_PATH: &str = "/tmp/irisos_fat32/fat32.fs";

pub fn setup() -> PathBuf {
    println!("Running setup script...");
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests/setup.sh");
    let output = Command::new("bash")
        .arg(d.display().to_string().as_str())
        .output()
        .expect("failed to execute setup script.");
    println!("Setup script output: {:?}", output);
    FS_PATH.into()
}

#[allow(dead_code)]
pub fn purge_fs() -> io::Result<()> {
    fs::remove_file("/tmp/irisos_fat32/fat32.fs").unwrap();
    fs::remove_dir("/tmp/irisos_fat32/").unwrap();
    Ok(())
}
