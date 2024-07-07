use super::super::mapper::{Mapper, MappingResult};
use crate::common::Device;

pub struct Mapper11 {
    /// select the 32kb bank
    prg_bank: u8,

    /// in 32kb units
    prg_count: u8,

    /// select the 8kb bank
    chr_bank: u8,

    /// in 8kb units
    chr_count: u8,

    /// is using CHR RAM?
    is_chr_ram: bool,
}

impl Mapper11 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            prg_count: 0,
            chr_bank: 0,
            chr_count: 0,
            is_chr_ram: true,
        }
    }

    fn map_ppu(&self, address: u16) -> MappingResult {
        let bank = self.chr_bank % self.chr_count;

        let start_of_bank = 0x2000 * bank as usize;

        MappingResult::Allowed(start_of_bank + (address & 0x1FFF) as usize)
    }
}

impl Mapper for Mapper11 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, chr_count: u8, _sram_count: u8) {
        // even and positive
        assert!(prg_count % 2 == 0 && prg_count > 0);

        self.prg_count = prg_count / 2;
        self.chr_count = chr_count;
        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::Cpu => match address {
                0x6000..=0x7FFF => MappingResult::Denied,
                0x8000..=0xFFFF => {
                    let bank = self.prg_bank % self.prg_count;

                    let start_of_bank = 0x8000 * bank as usize;

                    MappingResult::Allowed(start_of_bank + (address & 0x7FFF) as usize)
                }
                0x4020..=0x5FFF => MappingResult::Denied,
                _ => unreachable!(),
            },
            Device::Ppu => {
                if address < 0x2000 {
                    self.map_ppu(address)
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn map_write(&mut self, address: u16, data: u8, device: Device) -> MappingResult {
        match device {
            Device::Cpu => match address {
                0x6000..=0x7FFF => MappingResult::Denied,
                0x8000..=0xFFFF => {
                    self.prg_bank = data & 0x3;
                    self.chr_bank = (data >> 4) & 0xF;

                    MappingResult::Denied
                }
                0x4020..=0x5FFF => MappingResult::Denied,
                _ => unreachable!(),
            },
            Device::Ppu => {
                // CHR RAM
                if self.is_chr_ram && address <= 0x1FFF {
                    self.map_ppu(address)
                } else {
                    MappingResult::Denied
                }
            }
        }
    }

    fn save_state_size(&self) -> usize {
        5
    }

    fn save_state(&self) -> Vec<u8> {
        vec![
            self.prg_bank,
            self.prg_count,
            self.chr_bank,
            self.chr_count,
            self.is_chr_ram as u8,
        ]
    }

    fn load_state(&mut self, data: Vec<u8>) {
        self.prg_bank = data[0];
        self.prg_count = data[1];
        self.chr_bank = data[2];
        self.chr_count = data[3];
        self.is_chr_ram = data[4] != 0;
    }
}
