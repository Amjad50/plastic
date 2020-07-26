use crate::mapper::Mapper;
use common::{Device, MirroringMode};

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
    fn init(
        &mut self,
        prg_count: u8,
        is_chr_ram: bool,
        chr_count: u8,
        contain_sram: bool,
        _sram_count: u8,
    ) {
        assert!(!contain_sram, "Mapper 2 cannot have PRG ram");

        self.prg_count = prg_count;
        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> (bool, usize) {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => (false, 0),
                    0x8000..=0xFFFF => {
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
                        (true, start_of_bank + (address & 0x3FFF) as usize)
                    }
                    _ => unreachable!(),
                }
            }
            Device::PPU => {
                // it does not matter if its a ram or rom, same array location
                if address < 0x2000 {
                    // only one fixed memory
                    (true, address as usize)
                } else {
                    unreachable!()
                }
            }
        }
    }

    fn map_write(&mut self, address: u16, data: u8, device: Device) -> (bool, usize) {
        match device {
            Device::CPU => {
                // only accepts writes from CPU
                if address >= 0x8000 {
                    self.prg_top_bank = data;
                }
                (false, 0)
            }
            Device::PPU => {
                // CHR RAM
                if self.is_chr_ram && address >= 0x0000 && address <= 0x1FFF {
                    (true, address as usize)
                } else {
                    (false, 0)
                }
            }
        }
    }

    fn is_hardwired_mirrored(&self) -> bool {
        true
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        unreachable!()
    }

    fn is_irq_pin_state_changed_requested(&self) -> bool {
        false
    }

    fn irq_pin_state(&self) -> bool {
        unreachable!()
    }

    fn clear_irq_request_pin(&mut self) {}
}
