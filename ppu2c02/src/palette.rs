use common::{
    save_state::{Savable, SaveError},
    Bus, Device,
};

pub struct Palette {
    palette_data: [u8; 0x20],
}

impl Palette {
    pub fn new() -> Self {
        Self {
            palette_data: [
                0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D, 0x08, 0x10, 0x08, 0x24, 0x00, 0x00,
                0x04, 0x2C, 0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14, 0x08, 0x3A, 0x00, 0x02,
                0x00, 0x20, 0x2C, 0x08,
            ],
        }
    }

    pub fn map_address(address: u16) -> u8 {
        // mirror addresses 0x3F10/0x3F14/0x3F18/0x3F1C to 0x3F00/0x3F04/0x3F08/0x3F0C
        if address & 0x10 != 0 && address & 0b11 == 0 {
            (address & 0xF) as u8
        } else {
            (address & 0x1F) as u8
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for Palette {
    fn read(&self, address: u16, device: Device) -> u8 {
        assert!(device == Device::PPU && address >= 0x3F00 && address <= 0x3FFF);

        self.palette_data[Self::map_address(address) as usize]
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        assert!(device == Device::PPU && address >= 0x3F00 && address <= 0x3FFF);

        self.palette_data[Self::map_address(address) as usize] = data;
    }
}

impl Savable for Palette {
    fn save<W: std::io::Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_all(&self.palette_data)?;

        Ok(())
    }

    fn load<R: std::io::Read>(&mut self, reader: &mut R) -> Result<(), SaveError> {
        reader.read_exact(&mut self.palette_data)?;

        Ok(())
    }
}
