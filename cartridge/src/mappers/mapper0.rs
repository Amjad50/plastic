use crate::mapper::Mapper;
use common::{Device, MirroringMode};

pub struct Mapper0 {
    has_32kb_prg_rom: bool,
    is_chr_ram: bool,
    contain_sram: bool,
}

impl Mapper0 {
    pub fn new() -> Self {
        Self {
            has_32kb_prg_rom: false,
            is_chr_ram: false,
            contain_sram: false,
        }
    }
}

impl Mapper for Mapper0 {
    fn init(
        &mut self,
        prg_count: u8,
        is_chr_ram: bool,
        chr_count: u8,
        contain_sram: bool,
        _sram_count: u8,
    ) {
        // the only allowed options
        assert!(chr_count <= 1);
        assert!(prg_count == 1 || prg_count == 2);

        self.has_32kb_prg_rom = prg_count == 2;
        self.is_chr_ram = is_chr_ram;
        self.contain_sram = contain_sram;
    }

    fn map_read(&self, address: u16, device: Device) -> (bool, usize) {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => {
                        if self.contain_sram {
                            (true, address as usize & 0x1FFF)
                        } else {
                            (false, 0)
                        }
                    }
                    0x8000..=0xFFFF => {
                        // 0x7FFF is for mapping 0x8000-0xFFFF to 0x0000-0x7FFF
                        // which is the range of the array
                        (
                            true,
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

    fn map_write(&mut self, address: u16, _: u8, device: Device) -> (bool, usize) {
        // only for RAMs

        match device {
            Device::CPU => {
                if self.contain_sram && address >= 0x6000 && address <= 0x7FFF {
                    (true, address as usize & 0x1FFF)
                } else {
                    (false, 0)
                }
            }
            Device::PPU => {
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
