use crate::mapper::Mapper;
use common::{Device, MirroringMode};

// FIXME: add support for 512kb as now only support 256kb
pub struct Mapper1 {
    writing_shift_register: u8,

    /// 4bit0
    /// -----
    /// CPPMM
    /// |||||
    /// |||++- Mirroring (0: one-screen, lower bank; 1: one-screen, upper bank;
    /// |||               2: vertical; 3: horizontal)
    /// |++--- PRG ROM bank mode (0, 1: switch 32 KB at $8000, ignoring low bit of bank number;
    /// |                         2: fix first bank at $8000 and switch 16 KB bank at $C000;
    /// |                         3: fix last bank at $C000 and switch 16 KB bank at $8000)
    /// +----- CHR ROM bank mode (0: switch 8 KB at a time; 1: switch two separate 4 KB banks)
    control_register: u8,

    /// 4bit0
    /// -----
    /// CCCCC
    /// |||||
    /// +++++- Select 4 KB or 8 KB CHR bank at PPU $0000 (low bit ignored in 8 KB mode)
    chr_0_bank: u8,

    /// 4bit0
    /// -----
    /// CCCCC
    /// |||||
    /// +++++- Select 4 KB CHR bank at PPU $1000 (ignored in 8 KB mode)
    chr_1_bank: u8,

    /// 4bit0
    /// -----
    /// RPPPP
    /// |||||
    /// |++++- Select 16 KB PRG ROM bank (low bit ignored in 32 KB mode)
    /// +----- PRG RAM chip enable (0: enabled; 1: disabled; ignored on MMC1A)
    prg_bank: u8,

    /// in 4kb units
    chr_count: u8,

    /// in 16kb units
    prg_count: u8,
}

impl Mapper1 {
    pub fn new() -> Self {
        Self {
            writing_shift_register: 0b10000,
            control_register: 0,
            chr_0_bank: 0,
            chr_1_bank: 0,
            prg_bank: 0,

            chr_count: 0,
            prg_count: 0,
        }
    }

    fn reset_shift_register(&mut self) {
        // the 1 is used to indicate that the shift register is full when it
        // reaches the end
        self.writing_shift_register = 0b10000;
    }

    fn get_mirroring(&self) -> u8 {
        self.control_register & 0b00011
    }

    fn get_prg_bank(&self) -> u8 {
        self.prg_bank & 0b1111
    }

    fn is_prg_32kb_mode(&self) -> bool {
        self.control_register & 0b01000 == 0
    }

    /// this should be used in combination with `is_PRG_32kb_mode`
    /// this function will assume that the mapper is in 16kb mode
    /// if the first bank is fixed at 0x8000 and the second chunk of 16kb
    /// is switchable, this should return `true`
    /// if the last bank is fixed into 0xC000 and the first chunk of 16kb
    /// is switchable, this should return `false`
    fn is_first_prg_chunk_fixed(&self) -> bool {
        self.control_register & 0b00100 == 0
    }

    fn is_chr_8kb_mode(&self) -> bool {
        self.control_register & 0b10000 == 0
    }

    fn is_prg_ram_enabled(&self) -> bool {
        self.prg_bank & 0b10000 != 0
    }
}

impl Mapper for Mapper1 {
    fn init(&mut self, prg_count: u8, chr_count: u8) {
        self.prg_count = prg_count;
        self.chr_count = chr_count * 2; // since this passed as the number of 8kb banks

        self.reset_shift_register();
    }

    fn map_read(&self, address: u16, device: Device) -> usize {
        match device {
            Device::CPU => {
                let bank = if self.is_prg_32kb_mode() {
                    // ignore last bit
                    self.get_prg_bank() & 0b11110
                } else {
                    if address >= 0x8000 && address <= 0xBFFF {
                        if self.is_first_prg_chunk_fixed() {
                            0
                        } else {
                            self.get_prg_bank()
                        }
                    } else if address >= 0xC000 {
                        if self.is_first_prg_chunk_fixed() {
                            self.get_prg_bank()
                        } else {
                            // last bank
                            self.prg_count - 1
                        }
                    } else {
                        unreachable!();
                    }
                } as usize;

                assert!(bank <= self.prg_count as usize);

                let start_of_bank = 0x4000 * bank;

                let last_bank = 0x4000 * (self.prg_count - 1) as usize;

                // since banks can be odd in number, we don't want to go out
                // of bounds, but this solution does mirroring, in case of
                // a possible out of bounds, but not sure what is the correct
                // solution
                let mask = if self.is_prg_32kb_mode() && start_of_bank != last_bank {
                    0x7FFF
                } else {
                    0x3FFF
                };

                // add the offset
                start_of_bank + (address & mask) as usize
            }
            Device::PPU => {
                let bank = if self.is_chr_8kb_mode() {
                    self.chr_0_bank & 0b11110
                } else {
                    if address <= 0x0FFF {
                        self.chr_0_bank
                    } else if address >= 0x1000 && address <= 0x1FFF {
                        self.chr_1_bank
                    } else {
                        unreachable!()
                    }
                } as usize;

                // let bank = bank & (self.chr_count - 1) as usize;
                assert!(bank <= self.chr_count as usize);

                let start_of_bank = 0x1000 * bank;

                let mask = if self.is_chr_8kb_mode() {
                    0x1FFF
                } else {
                    0xFFF
                };

                // add the offset
                start_of_bank + (address & mask) as usize
            }
        }
    }

    fn map_write(&mut self, address: u16, data: u8, device: Device) {
        match device {
            Device::CPU => {
                // only accepts writes from CPU
                if address >= 0x8000 {
                    if data & 0x80 != 0 {
                        self.reset_shift_register();
                    } else {
                        let should_save = self.writing_shift_register & 1 != 0;
                        // shift
                        self.writing_shift_register >>= 1;
                        self.writing_shift_register |= (data & 1) << 4;

                        // reached the end, then save
                        if should_save {
                            let result = self.writing_shift_register & 0b11111;
                            match address {
                                0x8000..=0x9FFF => self.control_register = result,
                                0xA000..=0xBFFF => self.chr_0_bank = result,
                                0xC000..=0xDFFF => self.chr_1_bank = result,
                                0xE000..=0xFFFF => self.prg_bank = result,
                                _ => {
                                    unreachable!();
                                }
                            }

                            self.reset_shift_register();
                        }
                    }
                }
            }
            Device::PPU => {
                // CHR RAM
            }
        }
    }

    fn is_hardwired_mirrored(&self) -> bool {
        false
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        [
            MirroringMode::SingleScreenLowBank,
            MirroringMode::Horizontal,
            MirroringMode::Vertical,
            MirroringMode::Horizontal,
        ][self.get_mirroring() as usize]
    }

    fn is_irq_requested(&self) -> bool {
        false
    }

    fn clear_irq_request_pin(&mut self) {}
}
