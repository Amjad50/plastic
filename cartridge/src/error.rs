use std::{
    convert::From,
    default::Default,
    error::Error,
    fmt::{Debug, Display, Formatter, Result as fmtResult},
    io::{Error as ioError, ErrorKind},
};

pub enum CartridgeError {
    FileError(ioError),
    HeaderError,
    TooLargeFile(u64),
    ExtensionError,
    MapperNotImplemented(u8),
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
            Self::MapperNotImplemented(id) => format!("Mapper {} is not yet implemented", id),
            Self::ExtensionError => "The cartridge file must end with `.nes` extension".to_owned(),
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

pub enum SramError {
    NoSramFileFound,
    SramFileSizeDoesNotMatch,
    FailedToSaveSramFile,
    Others,
}

impl SramError {
    fn get_message(&self) -> &str {
        match self {
            Self::NoSramFileFound => "Could not load cartridge save file",
            Self::SramFileSizeDoesNotMatch => "There is a conflict in the size \
                                            of SRAM save file in the INES header and the file in disk",
            Self::FailedToSaveSramFile => "Could not save cartridge save file",
            Self::Others => "Unknown error occured while trying to save/load \
                          cartridge save file",
        }
    }
}

impl From<ioError> for SramError {
    fn from(from: ioError) -> Self {
        match from.kind() {
            ErrorKind::NotFound => Self::NoSramFileFound,
            ErrorKind::PermissionDenied => Self::FailedToSaveSramFile,
            _ => Self::Others,
        }
    }
}

impl Error for SramError {}

impl Display for SramError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl Debug for SramError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl Default for SramError {
    fn default() -> Self {
        Self::Others
    }
}
