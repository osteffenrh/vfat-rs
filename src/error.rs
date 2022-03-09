use alloc::string::String;
use binrw::io::ErrorKind;
use snafu::prelude::*;

pub type Result<T> = core::result::Result<T, VfatRsError>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum VfatRsError {
    #[snafu(display("MBR Error: {error}"))]
    Mbr { error: MbrError },
    #[snafu(display("TODO"))]
    MyWhatever,
    #[snafu(display("Free cluster not found, probably memory is full!?"))]
    FreeClusterNotFound,
    #[snafu(display("Checked mult failed."))]
    CheckedMulFailed,

    #[snafu(display("BinRW Error: {}", source))]
    BinRwError { source: BinRwErrorWrapper },

    #[snafu(display("Impossible delete non empty directory: {}", target))]
    NonEmptyDirectory { target: String },
    #[snafu(display("File not found: '{}'", target))]
    FileNotFound { target: String },
    #[snafu(display("Entry not found: '{}'", target))]
    EntryNotFound { target: String },
    #[snafu(display("Cannot delete pseudo directory: '{}'", target))]
    CannotDeletePseudoDir { target: String },
}

#[derive(Debug, Snafu)]
#[snafu(display("{value}"))]
pub struct BinRwErrorWrapper {
    pub(crate) value: binrw::error::Error,
}
impl From<binrw::error::Error> for BinRwErrorWrapper {
    fn from(value: binrw::Error) -> Self {
        Self { value }
    }
}
impl From<binrw::error::Error> for VfatRsError {
    fn from(err: binrw::Error) -> Self {
        VfatRsError::BinRwError {
            source: BinRwErrorWrapper { value: err },
        }
    }
}
impl From<binrw::io::Error> for VfatRsError {
    fn from(value: binrw::io::Error) -> Self {
        VfatRsError::from(binrw::Error::from(value))
    }
}
impl From<binrw::io::ErrorKind> for VfatRsError {
    fn from(value: ErrorKind) -> Self {
        VfatRsError::from(binrw::io::Error::from(value))
    }
}

#[derive(Debug, Snafu)]
pub enum MbrError {
    #[snafu(display("Not a fat32 partition: {index}"))]
    InvalidPartition { index: usize },
}

// Used for Impl Write/Read
impl From<VfatRsError> for binrw::io::Error {
    fn from(_err: VfatRsError) -> Self {
        // TODO: provide useful output
        binrw::io::ErrorKind::Other.into()
    }
}
