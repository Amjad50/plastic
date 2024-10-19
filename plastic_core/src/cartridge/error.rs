use std::{
    convert::From,
    default::Default,
    error::Error,
    fmt::{Debug, Display, Formatter, Result as fmtResult},
    io::{Error as ioError, ErrorKind},
};

/// Error happening when loading a NES cartridge.
pub enum CartridgeError {
    /// Error with file input/output.
    /// Contains an [`io::Error`][ioError] which provides more details about the error.
    FileError(ioError),

    /// The cartridge header is invalid or corrupted.
    HeaderError,

    /// The file size is too large.
    /// Contains the size of the file in bytes.
    TooLargeFile(u64),

    /// The file extension is not recognized or supported.
    ExtensionError,

    /// The mapper type is not implemented.
    MapperNotImplemented(u16),
}

impl CartridgeError {
    fn get_message(&self) -> String {
        match self {
            Self::FileError(err) => format!("FileError: {}", err),
            Self::HeaderError => "This is not a valid iNES file".to_owned(),
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
