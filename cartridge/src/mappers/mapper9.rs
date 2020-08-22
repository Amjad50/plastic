use crate::mapper::{Mapper, MappingResult};
use common::{Device, MirroringMode};
use std::cell::Cell;

pub struct Mapper9 {
    /// ($A000-$AFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxx PPPP
    ///      ||||
    ///      ++++- Select 8 KB PRG ROM bank for CPU $8000-$9FFF
    prg_bank: u8,

    /// selector for PPU $0000-$0FFF
    /// can have values 0xFD or 0xFE only
    latch_0: Cell<u8>,

    /// selector for PPU $1000-$1FFF
    /// can have values 0xFD or 0xFE only
    latch_1: Cell<u8>,

    /// ($B000-$BFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxC CCCC
    ///    | ||||
    ///    +-++++- Select 4 KB CHR ROM bank for PPU $0000-$0FFF
    ///            used when latch 0 = $FD
    chr_fd_0000_bank: u8,

    /// ($C000-$CFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxC CCCC
    ///    | ||||
    ///    +-++++- Select 4 KB CHR ROM bank for PPU $0000-$0FFF
    ///            used when latch 0 = $FE
    chr_fe_0000_bank: u8,

    /// ($D000-$DFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxC CCCC
    ///    | ||||
    ///    +-++++- Select 4 KB CHR ROM bank for PPU $1000-$1FFF
    ///            used when latch 1 = $FD
    chr_fd_1000_bank: u8,

    /// ($E000-$EFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxC CCCC
    ///    | ||||
    ///    +-++++- Select 4 KB CHR ROM bank for PPU $1000-$1FFF
    ///            used when latch 1 = $FE
    chr_fe_1000_bank: u8,

    /// ($F000-$FFFF)
    /// 7  bit  0
    /// ---- ----
    /// xxxx xxxM
    ///         |
    ///         +- Nametable mirroring (0: vertical; 1: horizontal)
    mirroring_vertical: bool,

    /// ($C000-$DFFE, even)
    /// the value to reload `irq_counter` when it reaches zero or when asked
    /// to be reloaded from `($C001-$DFFF, odd)`

    /// is using CHR RAM?
    is_chr_ram: bool,

    /// in 4kb units
    chr_count: u8,

    /// in 8kb units
    prg_count: u8,
}

impl Mapper9 {
    pub fn new() -> Self {
        Self {
            prg_bank: 0,
            latch_0: Cell::new(0xFE),
            latch_1: Cell::new(0xFE),
            chr_fd_0000_bank: 0,
            chr_fe_0000_bank: 0,
            chr_fd_1000_bank: 0,
            chr_fe_1000_bank: 0,
            mirroring_vertical: false,
            is_chr_ram: false,
            chr_count: 0,
            prg_count: 0,
        }
    }
}

impl Mapper for Mapper9 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, chr_count: u8, _sram_count: u8) {
        self.prg_count = prg_count * 2;

        // because 0xA000-0xFFFF holds the last 3 banks (fixed)
        assert!(self.prg_count > 3);

        self.chr_count = chr_count * 2;

        self.is_chr_ram = is_chr_ram;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::CPU => match address {
                0x6000..=0x7FFF => MappingResult::Allowed(address as usize & 0x1FFF),
                0x8000..=0xFFFF => {
                    let mut bank = match address {
                        0x8000..=0x9FFF => self.prg_bank,
                        0xA000..=0xFFFF => {
                            // last 3 banks
                            // 0: third last bank, 1: second last bank, 2: last bank
                            let bank = ((address >> 13) & 0b11) as u8 - 1;

                            self.prg_count - 3 + bank
                        }
                        _ => unreachable!(),
                    } as usize;

                    bank %= self.prg_count as usize;

                    let start_of_bank = bank * 0x2000;

                    MappingResult::Allowed(start_of_bank + (address & 0x1FFF) as usize)
                }
                _ => unreachable!(),
            },
            Device::PPU => {
                if address < 0x2000 {
                    let mut bank = if address & 0x1000 == 0 {
                        // set latch 0
                        if address == 0x0FD8 {
                            self.latch_0.set(0xFD);
                        } else if address == 0x0FE8 {
                            self.latch_0.set(0xFE);
                        }

                        match self.latch_0.get() {
                            0xFD => self.chr_fd_0000_bank,
                            0xFE => self.chr_fe_0000_bank,
                            _ => unreachable!(),
                        }
                    } else {
                        // set latch 1
                        if address & 0x8 != 0 {
                            let middle_byte = (address >> 4) & 0xFF;
                            if middle_byte == 0xFD || middle_byte == 0xFE {
                                self.latch_1.set(middle_byte as u8);
                            }
                        }

                        match self.latch_1.get() {
                            0xFD => self.chr_fd_1000_bank,
                            0xFE => self.chr_fe_1000_bank,
                            _ => unreachable!(),
                        }
                    } as usize;

                    bank %= self.chr_count as usize;

                    let start_of_bank = bank * 0x1000;

                    MappingResult::Allowed(start_of_bank + (address & 0xFFF) as usize)
                } else {
                    unreachable!();
                }
            }
        }
    }

    fn map_write(&mut self, address: u16, data: u8, device: Device) -> MappingResult {
        match device {
            Device::CPU => match address {
                0x6000..=0x7FFF => MappingResult::Allowed(address as usize & 0x1FFF),
                0x8000..=0xFFFF => {
                    match address {
                        0xA000..=0xAFFF => self.prg_bank = data & 0xF,
                        0xB000..=0xBFFF => self.chr_fd_0000_bank = data & 0x1F,
                        0xC000..=0xCFFF => self.chr_fe_0000_bank = data & 0x1F,
                        0xD000..=0xDFFF => self.chr_fd_1000_bank = data & 0x1F,
                        0xE000..=0xEFFF => self.chr_fe_1000_bank = data & 0x1F,
                        0xF000..=0xFFFF => self.mirroring_vertical = data & 1 == 0,
                        _ => {}
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

    fn is_hardwired_mirrored(&self) -> bool {
        false
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        if self.mirroring_vertical {
            MirroringMode::Vertical
        } else {
            MirroringMode::Horizontal
        }
    }
}
