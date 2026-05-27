use std::{fmt, io};

#[derive(Debug)]
pub enum CdcError {
    Io(io::Error),
    NoDevice,
    InvalidPath,
}

impl fmt::Display for CdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(io) => write!(f, "{io}"),
            Self::NoDevice => write!(
                f,
                "No usb cdc device found at path /dev/usbmodem* or /dev/tty*"
            ),
            Self::InvalidPath => {
                write!(f, "Provided path is not a valid fs path")
            }
        }
    }
}

impl From<io::Error> for CdcError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::error::Error for CdcError {}
