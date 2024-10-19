use std::convert::From;
use std::error::Error;
use std::fmt::Display;
use std::io::{Error as ioError, Read, Write};

pub trait Savable {
    fn save<W: Write>(&self, writer: &mut W) -> Result<(), SaveError>;
    fn load<R: Read>(&mut self, reader: &mut R) -> Result<(), SaveError>;
}

#[derive(Debug)]
pub enum SaveError {
    IoError(ioError),
    ContainExtraData,
    Others,
}

impl From<ioError> for SaveError {
    fn from(e: ioError) -> Self {
        SaveError::IoError(e)
    }
}

impl Error for SaveError {}

impl Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::IoError(err) => write!(f, "IO Error: {}", err),
            SaveError::ContainExtraData => {
                write!(f, "Contain Extra Data after the end of the file")
            }
            SaveError::Others => write!(f, "Others"),
        }
    }
}
