use binrw::io::ErrorKind;
use snafu::prelude::*;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("MBR Error: {error}"))]
    MbrError {
        error: MbrError,
    },
    #[snafu(display("TODO"))]
    MyWhatever,
    #[snafu(display("Free cluster not found, probably memory is full!?"))]
    FreeClusterNotFound,
    #[snafu(display("Checked mult failed."))]
    CheckedMulFailed,
    BinReadConvFailed {
        source: self::BinRwErrorWrapper,
    },
    BinRwError {
        source: BinRwErrorWrapper,
    },
}
#[derive(Debug, Snafu)]
#[snafu(display("BinRwErrorWrapper: {value}"))]
pub struct BinRwErrorWrapper {
    pub(crate) value: binrw::error::Error,
}
impl From<binrw::error::Error> for BinRwErrorWrapper {
    fn from(value: binrw::Error) -> Self {
        Self { value }
    }
}
impl From<binrw::error::Error> for Error {
    fn from(err: binrw::Error) -> Self {
        Error::BinRwError {
            source: BinRwErrorWrapper { value: err },
        }
    }
}
impl From<binrw::io::Error> for Error {
    fn from(value: binrw::io::Error) -> Self {
        Error::from(binrw::Error::from(value))
    }
}
impl From<binrw::io::ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error::from(binrw::io::Error::from(value))
    }
}

#[derive(Debug, Snafu)]
pub enum MbrError {
    #[snafu(display("Not a fat32 partition: {index}"))]
    InvalidPartition { index: usize },
}
// TODO: eventually remove?!
impl From<Error> for binrw::io::Error {
    fn from(err: Error) -> Self {
        binrw::io::ErrorKind::Other.into()
    }
}
