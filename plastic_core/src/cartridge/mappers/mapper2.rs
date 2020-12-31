use super::super::mapper::{Mapper, MappingResult};
use crate::common::Device;

pub struct Mapper2 {
    prg_top_bank: u8,

    /// in 16kb units
    prg_count: u8,

    is_chr_ram: bool,
}

impl Mapper2 {
    pub fn new() -> Self {
        Self {
            prg_top_bank: 0,
            prg_count: 0,
            is_chr_ram: false,
        }
    }
}

impl Mapper for Mapper2 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, _chr_count: u8, _sram_count: u8) {
        self.prg_count = prg_count;
        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => MappingResult::Denied,
                    0x8000..=0xFFFF => {
                        let mut bank = if address >= 0x8000 && address <= 0xBFFF {
                            self.prg_top_bank & 0xF
                        } else if address >= 0xC000 {
                            self.prg_count - 1
                        } else {
                            unreachable!();
                        } as usize;

                        bank %= self.prg_count as usize;

                        let start_of_bank = 0x4000 * bank;

                        // add the offset
                        MappingResult::Allowed(start_of_bank + (address & 0x3FFF) as usize)
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

    fn map_write(&mut self, address: u16, data: u8, device: Device) -> MappingResult {
        match device {
            Device::CPU => match address {
                0x6000..=0x7FFF => MappingResult::Denied,
                0x8000..=0xFFFF => {
                    self.prg_top_bank = data;
                    MappingResult::Denied
                }
                0x4020..=0x5FFF => MappingResult::Denied,
                _ => unreachable!(),
            },
            Device::PPU => {
                // CHR RAM
                if self.is_chr_ram && address <= 0x1FFF {
                    MappingResult::Allowed(address as usize)
                } else {
                    MappingResult::Denied
                }
            }
        }
    }

    fn save_state_size(&self) -> usize {
        3
    }

    fn save_state(&self) -> Vec<u8> {
        vec![self.prg_top_bank, self.prg_count, self.is_chr_ram as u8]
    }

    fn load_state(&mut self, data: Vec<u8>) {
        self.prg_top_bank = data[0];
        self.prg_count = data[1];
        self.is_chr_ram = data[2] != 0;
    }
}
