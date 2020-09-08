use crate::mapper::{Mapper, MappingResult};
use common::Device;

pub struct Mapper66 {
    /// in 8kb units
    chr_count: u8,

    /// ($8000-$FFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxx xxCC
    ///        ||
    ///        ++- Select 8 KB CHR ROM bank for PPU $0000-$1FFF
    chr_bank: u8,

    /// in 32kb units
    prg_count: u8,

    /// ($8000-$FFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxPP xxxx
    ///   ||
    ///   ||
    ///   ++------ Select 32 KB PRG ROM bank for CPU $8000-$FFFF
    prg_bank: u8,

    /// using CHR RAM
    is_chr_ram: bool,
}

impl Mapper66 {
    pub fn new() -> Self {
        Self {
            chr_count: 0,
            chr_bank: 0,
            prg_count: 0,
            prg_bank: 0,
            is_chr_ram: false,
        }
    }

    fn map_ppu(&self, address: u16) -> MappingResult {
        let bank = self.chr_bank % self.chr_count;

        let start_of_bank = 0x2000 * bank as usize;

        MappingResult::Allowed(start_of_bank + (address & 0x1FFF) as usize)
    }
}

impl Mapper for Mapper66 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, chr_count: u8, _sram_count: u8) {
        // even and more than 0
        assert!(prg_count % 2 == 0 && prg_count > 0);

        self.prg_count = prg_count / 2;
        self.chr_count = chr_count;
        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::CPU => match address {
                0x6000..=0x7FFF => MappingResult::Denied,
                0x8000..=0xFFFF => {
                    let bank = self.prg_bank % self.prg_count;

                    let start_of_bank = 0x8000 * bank as usize;

                    MappingResult::Allowed(start_of_bank + (address & 0x7FFF) as usize)
                }
                0x4020..=0x5FFF => MappingResult::Denied,
                _ => unreachable!(),
            },
            Device::PPU => {
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
            Device::CPU => match address {
                0x6000..=0x7FFF => MappingResult::Denied,
                0x8000..=0xFFFF => {
                    self.chr_bank = data & 0x3;
                    self.prg_bank = (data >> 4) & 0x3;

                    MappingResult::Denied
                }
                0x4020..=0x5FFF => MappingResult::Denied,
                _ => unreachable!(),
            },
            Device::PPU => {
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
            self.chr_count,
            self.chr_bank,
            self.prg_count,
            self.prg_bank,
            self.is_chr_ram as u8,
        ]
    }

    fn load_state(&mut self, data: Vec<u8>) {
        self.chr_count = data[0];
        self.chr_bank = data[1];
        self.prg_count = data[2];
        self.prg_bank = data[3];
        self.is_chr_ram = data[4] != 0;
    }
}
