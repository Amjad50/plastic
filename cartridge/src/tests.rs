#[cfg(test)]
mod cartridge_tests {
    use crate::{Cartridge, CartridgeError};
    use std::fs::File;

    #[test]
    fn test_ines1_cartridge_read() -> Result<(), CartridgeError> {
        let cartridge = Cartridge::from_file(File::open("./tests/roms/test_creation.nes")?)?;

        // make sure that all characters match, meaning we read all of them
        for c in cartridge.prg_data {
            assert_eq!(c, 0xFF);
        }

        // make sure that all characters match, meaning we read all of them
        for c in cartridge.chr_data {
            assert_eq!(c, 0xEE);
        }

        // test passed
        Ok(())
    }
}
