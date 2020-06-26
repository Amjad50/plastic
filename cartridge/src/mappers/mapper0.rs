use crate::mapper::Mapper;
use common::Device;

pub struct Mapper0 {
    has_32kb_prg_rom: bool,
}

impl Mapper0 {
    pub fn new() -> Self {
        Self {
            has_32kb_prg_rom: false,
        }
    }
}

impl Mapper for Mapper0 {
    fn init(&mut self, prg_count: u8, _chr_count: u8) {
        // the only allowed options
        assert!(_chr_count == 1);
        assert!(prg_count == 1 || prg_count == 2);

        self.has_32kb_prg_rom = prg_count == 2;
    }

    fn map_read(&self, address: u16, device: Device) -> u16 {
        match device {
            Device::CPU => {
                if address >= 0x8000 && address <= 0xFFFF {
                    if self.has_32kb_prg_rom {
                        address
                    } else {
                        address & 0xBFFF
                    }
                } else {
                    unreachable!()
                }
            }
            Device::PPU => {
                if address >= 0x0000 && address < 0x2000 {
                    // only one fixed memory
                    address
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn map_write(&self, _: u16, _: u8, _: Device) {
        // nothing
    }
}
