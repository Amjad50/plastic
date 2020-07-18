use std::{
    convert::From,
    default::Default,
    error::Error,
    fmt::{Debug, Display, Formatter, Result as fmtResult},
    io::Error as ioError,
};

pub enum CartridgeError {
    FileError(ioError),
    HeaderError,
    TooLargeFile(u64),
    Others,
}

impl CartridgeError {
    fn get_message(&self) -> String {
        match self {
            Self::FileError(err) => format!("FileError: {}", err),
            Self::HeaderError => "This is not a valid iNES file".to_owned(),
            Self::Others => {
                "An unknown error occurred while decoding/reading the cartridge".to_owned()
            }
            Self::TooLargeFile(size) => format!(
                "The cartridge reader read all the data needed, but the file \
                still has some data at the end with size {}-bytes",
                size
            ),
        }
    }
}

impl Error for CartridgeError {}

impl Display for CartridgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl Debug for CartridgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl From<ioError> for CartridgeError {
    fn from(from: ioError) -> Self {
        Self::FileError(from)
    }
}

impl Default for CartridgeError {
    fn default() -> Self {
        Self::Others
    }
}
