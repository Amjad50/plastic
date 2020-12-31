use super::super::mapper::{Mapper, MappingResult};
use crate::common::Device;

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
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, _chr_count: u8, _sram_count: u8) {
        // the only allowed options
        assert!(prg_count == 1 || prg_count == 2);

        self.has_32kb_prg_rom = prg_count == 2;
        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => MappingResult::Denied,
                    0x8000..=0xFFFF => {
                        // 0x7FFF is for mapping 0x8000-0xFFFF to 0x0000-0x7FFF
                        // which is the range of the array
                        MappingResult::Allowed(
                            (if self.has_32kb_prg_rom {
                                address & 0x7FFF
                            } else {
                                // in case of the array being half of the size (i.e.
                                // not 32KB, then the range of the address will be only
                                // 0x8000-0xBFFF, and 0xC000-0xFFFF will mirror the
                                // previous range
                                address & 0xBFFF & 0x7FFF
                            }) as usize,
                        )
                    }
                    0x4020..=0x5FFF => MappingResult::Denied,
                    _ => unreachable!(),
                }
            }
            Device::PPU => {
                // it does not matter if its a ram or rom, same array location
                if address < 0x2000 {
                    // only one fixed memory
                    MappingResult::Allowed(address as usize)
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn map_write(&mut self, address: u16, _: u8, device: Device) -> MappingResult {
        // only for RAMs

        match device {
            Device::CPU => MappingResult::Denied,
            Device::PPU => {
                if self.is_chr_ram && address <= 0x1FFF {
                    MappingResult::Allowed(address as usize)
                } else {
                    MappingResult::Denied
                }
            }
        }
    }

    fn save_state_size(&self) -> usize {
        1
    }

    fn save_state(&self) -> Vec<u8> {
        vec![(self.is_chr_ram as u8) << 1 | self.has_32kb_prg_rom as u8]
    }

    fn load_state(&mut self, data: Vec<u8>) {
        let state = data[0];

        self.is_chr_ram = state & 0b10 != 0;
        self.has_32kb_prg_rom = state & 1 != 0;
    }
}
