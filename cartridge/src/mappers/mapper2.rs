use crate::mapper::Mapper;
use common::{Device, MirroringMode};

pub struct Mapper2 {
    prg_top_bank: u8,

    /// in 16kb units
    prg_count: u8,
}

impl Mapper2 {
    pub fn new() -> Self {
        Self {
            prg_top_bank: 0,
            prg_count: 0,
        }
    }
}

impl Mapper for Mapper2 {
    fn init(&mut self, prg_count: u8, _chr_count: u8) {
        self.prg_count = prg_count;
    }

    fn map_read(&self, address: u16, device: Device) -> usize {
        match device {
            Device::CPU => {
                let bank = if address >= 0x8000 && address <= 0xBFFF {
                    self.prg_top_bank & 0xF
                } else if address >= 0xC000 {
                    self.prg_count - 1
                } else {
                    unreachable!();
                } as usize;

                assert!(bank <= self.prg_count as usize);

                let start_of_bank = 0x4000 * bank;

                // add the offset
                start_of_bank + (address & 0x3FFF) as usize
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

    fn map_write(&mut self, address: u16, data: u8, device: Device) {
        match device {
            Device::CPU => {
                // only accepts writes from CPU
                if address >= 0x8000 {
                    self.prg_top_bank = data;
                }
            }
            Device::PPU => {
                // CHR RAM
            }
        }
    }

    fn is_hardwired_mirrored(&self) -> bool {
        true
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        unreachable!()
    }
}
