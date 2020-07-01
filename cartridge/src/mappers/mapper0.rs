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
                // this is just for extra caution
                if address >= 0x8000 && address <= 0xFFFF {
                    // 0x7FFF is for mapping 0x8000-0xFFFF to 0x0000-0x7FFF
                    // which is the range of the array
                    if self.has_32kb_prg_rom {
                        address & 0x7FFF
                    } else {
                        // in case of the array being half of the size (i.e.
                        // not 32KB, then the range of the address will be only
                        // 0x8000-0xBFFF, and 0xC000-0xFFFF will mirror the
                        // previous range
                        address & 0xBFFF & 0x7FFF
                    }
                } else {
                    unreachable!()
                }
            }
            Device::PPU => {
                // this is just for extra caution
                if address >= 0x0000 && address < 0x2000 {
                    // only one fixed memory
                    address
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn map_write(&mut self, _: u16, _: u8, _: Device) {
        // nothing
    }
}
