#[cfg(test)]
mod cartridge_tests {
    use crate::{Cartridge, CartridgeError};

    #[test]
    fn cartridge_file_not_found() {
        let err = Cartridge::from_file("./file/does/not/exists.nes")
            .err()
            .expect("Should get an error as the cartridge file does not exists");

        if let CartridgeError::FileError(file_err) = err {
            assert_eq!(file_err.kind(), std::io::ErrorKind::NotFound);
        } else {
            panic!("Should get file not found error");
        }
    }

    #[test]
    fn cartridge_extension_error_file() {
        let err = Cartridge::from_file("./file/does/not/exists.notnes")
            .err()
            .expect("Should get an error as the cartridge has extension error");

        if let CartridgeError::ExtensionError = err {
            // passed
        } else {
            panic!("Should get extension error");
        }
    }

    #[test]
    fn cartridge_wrong_header() {
        let err = Cartridge::from_file("./tests/roms/test_wrong_header.nes")
            .err()
            .expect("Should get an error as the cartridge has wrong header");

        if let CartridgeError::HeaderError = err {
            // passed
        } else {
            panic!("Should get header error");
        }
    }

    #[test]
    fn cartridge_large_file() {
        let err = Cartridge::from_file("./tests/roms/test_large_file.nes")
            .err()
            .expect("Should get an error as the cartridge file is larger than expected");

        if let CartridgeError::TooLargeFile(exceeded_size) = err {
            assert_eq!(exceeded_size, 1);
        } else {
            panic!("Should get too large file error");
        }
    }

    #[test]
    fn test_ines1_cartridge_read() -> Result<(), CartridgeError> {
        let cartridge = Cartridge::from_file("./tests/roms/test_creation.nes")?;

        // make sure that all characters match, meaning we read all of them
        for &c in &cartridge.prg_data {
            assert_eq!(c, 0xFF);
        }

        // make sure that all characters match, meaning we read all of them
        for &c in &cartridge.chr_data {
            assert_eq!(c, 0xEE);
        }

        // test passed
        Ok(())
    }
}
