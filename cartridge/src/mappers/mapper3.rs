use crate::mapper::{Mapper, MappingResult};
use common::Device;

pub struct Mapper3 {
    has_32kb_prg_rom: bool,

    chr_bank: u8,

    chr_count: u8,

    is_chr_ram: bool,
}

impl Mapper3 {
    pub fn new() -> Self {
        Self {
            has_32kb_prg_rom: false,
            chr_bank: 0,
            chr_count: 0,
            is_chr_ram: false,
        }
    }
}

impl Mapper for Mapper3 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, chr_count: u8, _sram_count: u8) {
        assert!(prg_count == 1 || prg_count == 2);

        self.has_32kb_prg_rom = prg_count == 2;
        self.chr_count = chr_count;
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
                    _ => unreachable!(),
                }
            }
            Device::PPU => {
                if address < 0x2000 {
                    let bank = self.chr_bank % self.chr_count;

                    let start_of_bank = 0x2000 * bank as usize;

                    MappingResult::Allowed(start_of_bank + (address & 0x1FFF) as usize)
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
                    if self.chr_count <= 4 {
                        // Maybe expecting CNROM mode, which is taking only the
                        // first 2 bits, because some games write bits on the
                        // leftmost as well which would result in an overflow
                        self.chr_bank = data & 0b11;
                    } else {
                        self.chr_bank = data;
                    }

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
}
