use crate::mapper::{BankMapping, BankMappingType, Mapper};
use common::MirroringMode;

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
    ///
    /// OR
    ///
    /// 4bit0
    /// -----
    /// ExxxC
    /// |   |
    /// |   +- Select 4 KB CHR RAM bank at PPU $0000 (ignored in 8 KB mode)
    /// +----- PRG RAM disable (0: enable, 1: open bus)
    ///
    /// OR
    ///
    /// 4bit0
    /// -----
    /// PSSxC
    /// ||| |
    /// ||| +- Select 4 KB CHR RAM bank at PPU $0000 (ignored in 8 KB mode)
    /// |++--- Select 8 KB PRG RAM bank
    /// +----- Select 256 KB PRG ROM bank
    chr_0_bank: u8,

    /// 4bit0
    /// -----
    /// CCCCC
    /// |||||
    /// +++++- Select 4 KB CHR bank at PPU $1000 (ignored in 8 KB mode)
    ///
    /// OR
    ///
    /// 4bit0
    /// -----
    /// ExxxC
    /// |   |
    /// |   +- Select 4 KB CHR RAM bank at PPU $0000 (ignored in 8 KB mode)
    /// +----- PRG RAM disable (0: enable, 1: open bus) (ignored in 8 KB mode)
    ///
    /// OR
    ///
    /// 4bit0
    /// -----
    /// PSSxC
    /// ||| |
    /// ||| +- Select 4 KB CHR RAM bank at PPU $0000 (ignored in 8 KB mode)
    /// |++--- Select 8 KB PRG RAM bank (ignored in 8 KB mode)
    /// +----- Select 256 KB PRG ROM bank (ignored in 8 KB mode)
    chr_1_bank: u8,

    /// 4bit0
    /// -----
    /// -PPPP
    ///  ||||
    ///  ++++- Select 16 KB PRG ROM bank (low bit ignored in 32 KB mode)
    prg_bank: u8,

    /// 4bit0
    /// -----
    /// R----
    /// |
    /// +----- PRG RAM chip enable (0: enabled; 1: disabled; ignored on MMC1A)
    prg_ram_enable: bool,

    /// is using CHR ram
    is_chr_ram: bool,

    /// in 4kb units
    chr_count: u8,

    /// in 16kb units
    prg_count: u8,

    /// in 8kb units
    prg_ram_count: u8,
}

impl Mapper1 {
    pub fn new() -> Self {
        Self {
            writing_shift_register: 0b10000,
            control_register: 0x0C, // power-up
            chr_0_bank: 0,
            chr_1_bank: 0,
            prg_bank: 0,

            prg_ram_enable: false,

            is_chr_ram: false,

            chr_count: 0,
            prg_count: 0,

            prg_ram_count: 0,
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
        // 8KB (SNROM) and not in 512KB PRG mode
        let snrom_prg_ram_enabled = if self.chr_count == 2 && self.prg_count <= 16 {
            if self.is_chr_8kb_mode() {
                self.chr_0_bank & 0x10 == 0
            } else {
                self.chr_1_bank & 0x10 == 0
            }
        } else {
            // only depend on `self.prg_ram_enable`
            true
        };

        self.prg_ram_enable && snrom_prg_ram_enabled
    }

    fn get_ppu_bank(&self, is_low_bank: bool) -> u16 {
        (if self.is_chr_8kb_mode() {
            // if its not the low bank, then take the next bank
            (self.chr_0_bank & 0b11110) + !is_low_bank as u8
        } else {
            if is_low_bank {
                self.chr_0_bank
            } else {
                self.chr_1_bank
            }
        }) as u16
    }

    fn get_prg_rom_bank(&self, is_low_bank: bool) -> u16 {
        let mut bank = if self.is_prg_32kb_mode() {
            // if its not a low bank, then add one to it to get the next bank
            (self.get_prg_bank() & 0b11110) + !is_low_bank as u8
        } else {
            if is_low_bank {
                if self.is_first_prg_chunk_fixed() {
                    0
                } else {
                    self.get_prg_bank()
                }
            } else {
                if self.is_first_prg_chunk_fixed() {
                    self.get_prg_bank()
                } else {
                    // last bank
                    self.prg_count - 1
                }
            }
        } as u16;

        if self.prg_count > 16 && self.chr_count == 2 {
            let prg_high_bit_512_mode = if self.is_chr_8kb_mode() {
                self.chr_0_bank & 0x10
            } else {
                self.chr_1_bank & 0x10
            } as u16;

            bank |= prg_high_bit_512_mode;
        }

        bank
    }

    fn get_prg_ram_bank(&self) -> u16 {
        (if self.prg_ram_count > 1 {
            if self.is_chr_8kb_mode() {
                (self.chr_0_bank >> 2) & 0x3
            } else {
                (self.chr_1_bank >> 2) & 0x3
            }
        } else {
            0
        }) as u16
    }

    fn get_mappings(&self) -> Vec<(BankMappingType, u8, BankMapping)> {
        let is_prg_ram_enabled = self.is_prg_ram_enabled() && self.prg_ram_count > 0;
        vec![
            (
                BankMappingType::CpuRam,
                0,
                BankMapping {
                    ty: BankMappingType::CpuRam,
                    to: self.get_prg_ram_bank(),
                    read: is_prg_ram_enabled,
                    write: is_prg_ram_enabled,
                },
            ),
            (
                BankMappingType::CpuRom,
                0,
                BankMapping {
                    ty: BankMappingType::CpuRom,
                    to: self.get_prg_rom_bank(true),
                    read: true,
                    write: false,
                },
            ),
            (
                BankMappingType::CpuRom,
                1,
                BankMapping {
                    ty: BankMappingType::CpuRom,
                    to: self.get_prg_rom_bank(false),
                    read: true,
                    write: false,
                },
            ),
            (
                BankMappingType::Ppu,
                0,
                BankMapping {
                    ty: BankMappingType::Ppu,
                    to: self.get_ppu_bank(true),
                    read: true,
                    write: self.is_chr_ram,
                },
            ),
            (
                BankMappingType::Ppu,
                1,
                BankMapping {
                    ty: BankMappingType::Ppu,
                    to: self.get_ppu_bank(false),
                    read: true,
                    write: self.is_chr_ram,
                },
            ),
        ]
    }
}

impl Mapper for Mapper1 {
    fn init(
        &mut self,
        prg_count: u16,
        is_chr_ram: bool,
        chr_count: u16,
        sram_count: u16,
    ) -> Vec<(BankMappingType, u8, BankMapping)> {
        self.prg_count = prg_count as u8;
        self.chr_count = chr_count as u8;
        self.is_chr_ram = is_chr_ram;

        self.prg_bank = prg_count as u8 - 1; // power-up, should be all set?
        self.control_register = 0b11100; // power-up state

        self.prg_ram_count = sram_count as u8;

        self.reset_shift_register();

        self.get_mappings()
    }

    fn write_register(
        &mut self,
        address: u16,
        data: u8,
    ) -> Vec<(BankMappingType, u8, BankMapping)> {
        let mut ret_val = Vec::new();

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
                    0xE000..=0xFFFF => {
                        self.prg_bank = result & 0xF;
                        self.prg_ram_enable = result & 0x10 == 0;
                    }
                    _ => {
                        unreachable!();
                    }
                }

                ret_val.extend(self.get_mappings());

                self.reset_shift_register();
            }
        }

        ret_val
    }

    fn cpu_ram_bank_size(&self) -> u16 {
        0x2000
    }

    fn cpu_rom_bank_size(&self) -> u16 {
        0x4000
    }

    fn ppu_bank_size(&self) -> u16 {
        0x1000
    }

    fn registers_memory_range(&self) -> std::ops::RangeInclusive<u16> {
        0x8000..=0xFFFF
    }

    fn is_hardwired_mirrored(&self) -> bool {
        false
    }

    fn nametable_mirroring(&self) -> MirroringMode {
        [
            MirroringMode::SingleScreenLowBank,
            MirroringMode::SingleScreenHighBank,
            MirroringMode::Vertical,
            MirroringMode::Horizontal,
        ][self.get_mirroring() as usize]
    }
}
