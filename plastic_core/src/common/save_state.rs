use std::convert::From;
use std::error::Error;
use std::fmt::Display;
use std::io::{Error as ioError, Read, Write};

pub trait Savable {
    fn save<W: Write>(&self, writer: &mut W) -> Result<(), SaveError>;
    fn load<R: Read>(&mut self, reader: &mut R) -> Result<(), SaveError>;
}

/// Error happening when saving/loading a state
#[derive(Debug)]
pub enum SaveError {
    /// Error with file input/output.
    /// Contains an [`io::Error`][ioError] which provides more details about the error.
    IoError(ioError),
    /// Contain Extra Data after the end of the file
    ContainExtraData,
    /// Error happened during serialization/deserialization, faulty data
    SerializationError,
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
            SaveError::SerializationError => write!(f, "Serialization Error"),
        }
    }
}
