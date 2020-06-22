extern crate cartridge;
use cartridge::{Cartridge, CartridgeError};
use std::fs::File;

#[test]
fn cartridge_load_ines_file() -> Result<(), CartridgeError>{
    let _ = Cartridge::from_file(File::open("./tests/roms/any_rom.nes")?)?;
    // test passed
    Ok(())
}