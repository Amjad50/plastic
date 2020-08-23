use crate::mapper::{Mapper, MappingResult};
use common::{Device, MirroringMode};
use std::cell::Cell;

pub struct Mapper4 {
    /// ($8000-$9FFE, even)
    /// 7  bit  0
    /// ---- ----
    /// xxxx xRRR
    ///       |||
    ///       +++- Specify which bank register to update on next write to Bank Data register
    ///              000: R0: Select 2 KB CHR bank at PPU $0000-$07FF (or $1000-$17FF)
    ///              001: R1: Select 2 KB CHR bank at PPU $0800-$0FFF (or $1800-$1FFF)
    ///              010: R2: Select 1 KB CHR bank at PPU $1000-$13FF (or $0000-$03FF)
    ///              011: R3: Select 1 KB CHR bank at PPU $1400-$17FF (or $0400-$07FF)
    ///              100: R4: Select 1 KB CHR bank at PPU $1800-$1BFF (or $0800-$0BFF)
    ///              101: R5: Select 1 KB CHR bank at PPU $1C00-$1FFF (or $0C00-$0FFF)
    ///              110: R6: Select 8 KB PRG ROM bank at $8000-$9FFF (or $C000-$DFFF)
    ///              111: R7: Select 8 KB PRG ROM bank at $A000-$BFFF
    bank_select: u8,

    /// ($8000-$9FFE, even)
    /// 7  bit  0
    /// ---- ----
    /// xPxx xxxx
    ///  +-------- PRG ROM bank mode (0: $8000-$9FFF swappable,
    ///                                  $C000-$DFFF fixed to second-last bank;
    ///                               1: $C000-$DFFF swappable,
    ///                                  $8000-$9FFF fixed to second-last bank)
    ///
    /// its either 8000-9FFF is fixed to the second-last bank and C000-DFFF
    /// is switchable, or the other way around
    ///
    /// true:  fix 8000
    /// false: fix C000
    prg_rom_bank_fix_8000: bool,

    /// the currently selected bank for the switchable of 8000-9FFF or C000-DFFF
    prg_bank_8000_c000: u8,

    /// the currently selected bank for A000-BFFF
    prg_bank_a000: u8,

    /// ($8000-$9FFE, even)
    /// 7  bit  0
    /// ---- ----
    /// Cxxx xxxx
    /// +--------- CHR A12 inversion (0: two 2 KB banks at $0000-$0FFF,
    ///                                  four 1 KB banks at $1000-$1FFF;
    ///                               1: two 2 KB banks at $1000-$1FFF,
    ///                                  four 1 KB banks at $0000-$0FFF)
    ///
    /// its either 0000-0FFF contain two 2kb banks and 1000-1FFF contain
    /// four 1kb banks or the other way around
    ///
    /// true:  use 2kb banks for 0000-07FF and 0800-0FFF
    /// false: use 2kb banks for 1000-17FF and 1800-1FFF
    chr_bank_2k_1000: bool,

    // chr banks
    chr_bank_r0: u8,
    chr_bank_r1: u8,
    chr_bank_r2: u8,
    chr_bank_r3: u8,
    chr_bank_r4: u8,
    chr_bank_r5: u8,

    /// ($A000-$BFFE, even)
    /// 7  bit  0
    /// ---- ----
    /// xxxx xxxM
    ///         |
    ///         +- Nametable mirroring (0: vertical; 1: horizontal)
    mirroring_vertical: bool,

    /// 7  bit  0
    /// ---- ----
    /// RWxx xxxx
    ///  |
    ///  +-------- Write protection (0: allow writes; 1: deny writes)
    prg_ram_allow_writes: bool,

    /// 7  bit  0
    /// ---- ----
    /// Rxxx xxxx
    /// |
    /// +--------- PRG RAM chip enable (0: disable; 1: enable)
    prg_ram_enabled: bool,

    /// ($C000-$DFFE, even)
    /// the value to reload `irq_counter` when it reaches zero or when asked
    /// to be reloaded from `($C001-$DFFF, odd)`
    irq_latch: u8,

    /// counter will be decremented, and when reached zero and `irq_enabled`
    /// `true` it should trigger an **IRQ** interrupt
    irq_counter: Cell<u8>,

    /// reload IRQ counter at the NEXT clocking of the IRQ
    reload_irq_counter_flag: Cell<bool>,

    /// denotes if an **IRQ** interrupt should occur on `irq_counter` reaching
    /// zero or not
    irq_enabled: bool,

    /// the status of the IRQ pin, should be used with `is_irq_pin_changed`
    irq_pin: Cell<bool>,

    /// indicate whether there is a change that the CPU should be notified of
    /// in the IRQ line, since the IRQ does not happen immediatly like NMI,
    /// it can be disabled when the `irq_pin` is set and then it is cleared
    ///
    /// in the NES, this is a wire connection directly from `irq_pin` to the CPU,
    /// but in this emulator, the CPU has its own copy of `irq_pin` so it should
    /// be notified if any changes happened from this side
    is_irq_pin_changed: Cell<bool>,

    /// is using CHR RAM?
    is_chr_ram: bool,

    /// in 1kb units
    chr_count: u16,

    /// in 8kb units
    prg_count: u8,

    /// false if the last accessed pattern table address is $0000
    /// true  if the last accessed pattern table address is $1000
    last_pattern_table: Cell<bool>,

    /// is PRG ram present?
    has_prg_ram: bool,
}

impl Mapper4 {
    pub fn new() -> Self {
        Self {
            bank_select: 0,
            prg_rom_bank_fix_8000: false,
            prg_bank_8000_c000: 0,
            prg_bank_a000: 0,
            chr_bank_2k_1000: false,
            chr_bank_r0: 0,
            chr_bank_r1: 0,
            chr_bank_r2: 0,
            chr_bank_r3: 0,
            chr_bank_r4: 0,
            chr_bank_r5: 0,
            mirroring_vertical: false,
            prg_ram_allow_writes: true,
            prg_ram_enabled: true,
            irq_latch: 0,
            irq_counter: Cell::new(0),
            reload_irq_counter_flag: Cell::new(false),
            irq_enabled: false,
            irq_pin: Cell::new(false),
            is_irq_pin_changed: Cell::new(false),
            is_chr_ram: false,
            chr_count: 0,
            prg_count: 0,
            last_pattern_table: Cell::new(false),
            has_prg_ram: false,
        }
    }

    fn handle_irq_counter(&self, address: u16) {
        let current_pattern_table = address & (1 << 12) != 0;

        // transition from 0 to 1
        if !self.last_pattern_table.get() && current_pattern_table {
            if self.reload_irq_counter_flag.get() || self.irq_counter.get() == 0 {
                self.reload_irq_counter_flag.set(false);
                self.irq_counter.set(self.irq_latch);
            } else {
                self.irq_counter
                    .set(self.irq_counter.get().saturating_sub(1));
            }

            if self.irq_counter.get() == 0 && self.irq_enabled {
                // trigger IRQ
                self.irq_pin.set(true);
                self.is_irq_pin_changed.set(true);
            }
        }

        self.last_pattern_table.set(current_pattern_table);
    }

    fn map_ppu(&self, address: u16) -> MappingResult {
        self.handle_irq_counter(address);

        let is_2k = (address & 0x1000 == 0) ^ self.chr_bank_2k_1000;

        let mut bank = if is_2k {
            if address & 0x0800 == 0 {
                self.chr_bank_r0
            } else {
                self.chr_bank_r1
            }
        } else {
            match (address >> 10) & 0b11 {
                0 => self.chr_bank_r2,
                1 => self.chr_bank_r3,
                2 => self.chr_bank_r4,
                3 => self.chr_bank_r5,
                _ => unreachable!(),
            }
        } as usize;

        bank %= self.chr_count as usize;

        let mask = if is_2k { 0x7FF } else { 0x3FF };

        let start_of_bank = bank * 0x400;

        MappingResult::Allowed(start_of_bank + (address & mask) as usize)
    }
}

impl Mapper for Mapper4 {
    fn init(&mut self, prg_count: u8, is_chr_ram: bool, chr_count: u8, sram_count: u8) {
        self.prg_count = prg_count * 2;
        self.chr_count = chr_count as u16 * 8;

        self.is_chr_ram = is_chr_ram;

        self.has_prg_ram = sram_count != 0;
    }

    fn map_read(&self, address: u16, device: Device) -> MappingResult {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => {
                        if self.prg_ram_enabled && self.has_prg_ram {
                            MappingResult::Allowed(address as usize & 0x1FFF)
                        } else {
                            MappingResult::Denied
                        }
                    }
                    0x8000..=0xFFFF => {
                        let mut bank = match address {
                            0x8000..=0x9FFF => {
                                if self.prg_rom_bank_fix_8000 {
                                    // second to last
                                    self.prg_count - 2
                                } else {
                                    self.prg_bank_8000_c000
                                }
                            }
                            0xA000..=0xBFFF => self.prg_bank_a000,
                            0xC000..=0xDFFF => {
                                if !self.prg_rom_bank_fix_8000 {
                                    // second to last
                                    self.prg_count - 2
                                } else {
                                    self.prg_bank_8000_c000
                                }
                            }
                            0xE000..=0xFFFF => self.prg_count - 1,
                            _ => unreachable!(),
                        } as usize;

                        bank %= self.prg_count as usize;

                        let start_of_bank = bank * 0x2000;

                        MappingResult::Allowed(start_of_bank + (address & 0x1FFF) as usize)
                    }
                    _ => unreachable!(),
                }
            }
            Device::PPU => {
                if address < 0x2000 {
                    self.map_ppu(address)
                } else {
                    unreachable!();
                }
            }
        }
    }

    fn map_write(&mut self, address: u16, data: u8, device: Device) -> MappingResult {
        match device {
            Device::CPU => {
                match address {
                    0x6000..=0x7FFF => {
                        if self.prg_ram_enabled && self.prg_ram_allow_writes && self.has_prg_ram {
                            MappingResult::Allowed(address as usize & 0x1FFF)
                        } else {
                            MappingResult::Denied
                        }
                    }
                    0x8000..=0xFFFF => {
                        match address {
                            0x8000..=0x9FFF => {
                                if address & 1 == 0 {
                                    // even
                                    self.bank_select = data & 0b111;
                                    self.prg_rom_bank_fix_8000 = data & 0x40 != 0;
                                    self.chr_bank_2k_1000 = data & 0x80 != 0;
                                } else {
                                    // odd
                                    match self.bank_select {
                                        0 => self.chr_bank_r0 = data & !(1), // store as even number
                                        1 => self.chr_bank_r1 = data & !(1), // store as even number
                                        2 => self.chr_bank_r2 = data,
                                        3 => self.chr_bank_r3 = data,
                                        4 => self.chr_bank_r4 = data,
                                        5 => self.chr_bank_r5 = data,
                                        6 => self.prg_bank_8000_c000 = data,
                                        7 => self.prg_bank_a000 = data,
                                        _ => unreachable!(),
                                    }
                                }
                            }
                            0xA000..=0xBFFF => {
                                if address & 1 == 0 {
                                    // even
                                    self.mirroring_vertical = data & 1 == 0;
                                } else {
                                    // odd
                                    // PRG RAM stuff
                                    self.prg_ram_allow_writes = data & 0x40 == 0;
                                    self.prg_ram_enabled = data & 0x80 != 0;
                                }
                            }
                            0xC000..=0xDFFF => {
                                if address & 1 == 0 {
                                    // even
                                    self.irq_latch = data;
                                } else {
                                    // odd
                                    self.reload_irq_counter_flag.set(true);
                                }
                            }
                            0xE000..=0xFFFF => {
                                // enable on odd addresses, disable on even addresses
                                self.irq_enabled = address & 1 != 0;

                                // if cleared, then clear the pin as well if it is set
                                // and notify the CPU
                                if !self.irq_enabled {
                                    self.irq_pin.set(false);
                                    self.is_irq_pin_changed.set(true);
                                }
                            }
                            _ => unreachable!(),
                        }

                        MappingResult::Denied
                    }
                    _ => unreachable!(),
                }
            }
            Device::PPU => {
                // CHR RAM
                if self.is_chr_ram && address <= 0x1FFF {
                    self.map_ppu(address)
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

    fn is_irq_pin_state_changed_requested(&self) -> bool {
        self.is_irq_pin_changed.get()
    }

    fn irq_pin_state(&self) -> bool {
        self.irq_pin.get()
    }

    fn clear_irq_request_pin(&mut self) {
        self.irq_pin.set(false);
        self.is_irq_pin_changed.set(false);
    }
}
