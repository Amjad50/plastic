use crate::mapper::Mapper;
use common::{Device, MirroringMode};

pub struct Mapper0 {
    has_32kb_prg_rom: bool,
    is_chr_ram: bool,
}

impl Mapper0 {
    pub fn new() -> Self {
        Self {
            has_32kb_prg_rom: false,
            is_chr_ram: false,
        }
    }
}

impl Mapper for Mapper0 {
    fn init(&mut self, prg_count: u8, chr_count: u8) {
        // the only allowed options
        assert!(chr_count <= 1);
        assert!(prg_count == 1 || prg_count == 2);

        self.has_32kb_prg_rom = prg_count == 2;
        self.is_chr_ram = chr_count == 0;
    }

    fn map_read(&self, address: u16, device: Device) -> usize {
        match device {
            Device::CPU => {
                if address >= 0x8000 {
                    // 0x7FFF is for mapping 0x8000-0xFFFF to 0x0000-0x7FFF
                    // which is the range of the array
                    (if self.has_32kb_prg_rom {
                        address & 0x7FFF
                    } else {
                        // in case of the array being half of the size (i.e.
                        // not 32KB, then the range of the address will be only
                        // 0x8000-0xBFFF, and 0xC000-0xFFFF will mirror the
                        // previous range
                        address & 0xBFFF & 0x7FFF
                    }) as usize
                } else {
                    unreachable!()
                }
            }
            Device::PPU => {
                // it does not matter if its a ram or rom, same array location
                if address < 0x2000 {
                    // only one fixed memory
                    address as usize
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn map_write(&mut self, _: u16, _: u8, _: Device) {
        // nothing
    }

    fn is_hardwired_mirrored(&self) -> bool {
        true
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        unreachable!()
    }

    fn is_irq_requested(&self) -> bool {
        false
    }

    fn clear_irq_request_pin(&mut self) {}
}
