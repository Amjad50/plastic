use crate::mapper::{Mapper, MappingResult};
use common::{Device, MirroringMode};

pub struct Mapper7 {
    /// select the 32KB bank
    prg_bank: u8,

    /// in 32kb units
    prg_count: u8,

    /// this mapper support only one screen mirroring and able to switch between
    /// low and high banks
    ///
    /// false: low, true: high
    is_mirroring_screen_high_bank: bool,

    is_chr_ram: bool,
}

impl Mapper7 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            prg_count: 0,
            is_mirroring_screen_high_bank: false,
            is_chr_ram: false,
        }
    }
}

impl Mapper for Mapper7 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, _chr_count: u8, _sram_count: u8) {
        // even and positive
        assert!(prg_count & 1 == 0 && prg_count > 0);

        self.prg_count = prg_count / 2;
        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => MappingResult::Allowed(address as usize & 0x1FFF),
                    0x8000..=0xFFFF => {
                        let bank = self.prg_bank as usize;

                        assert!(bank < self.prg_count as usize);

                        let start_of_bank = bank * 0x8000;

                        // add the offset
                        MappingResult::Allowed(start_of_bank + (address & 0x7FFF) as usize)
                    }
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
                0x6000..=0x7FFF => MappingResult::Allowed(address as usize & 0x1FFF),
                0x8000..=0xFFFF => {
                    self.prg_bank = data & 0x7;
                    self.is_mirroring_screen_high_bank = data & 0x10 != 0;

                    MappingResult::Denied
                }
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

    fn is_hardwired_mirrored(&self) -> bool {
        false
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        if self.is_mirroring_screen_high_bank {
            MirroringMode::SingleScreenHighBank
        } else {
            MirroringMode::SingleScreenLowBank
        }
    }
}
