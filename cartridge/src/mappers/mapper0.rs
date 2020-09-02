use crate::mapper::{BankMapping, BankMappingType, Mapper};

pub struct Mapper0 {}

impl Mapper0 {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mapper for Mapper0 {
    fn init(
        &mut self,
        prg_count: u8,
        is_chr_ram: bool,
        _chr_count: u8,
        sram_count: u8,
    ) -> Vec<(BankMappingType, u8, BankMapping)> {
        // the only allowed options
        assert!(prg_count == 1 || prg_count == 2);

        let has_32kb_prg_rom = prg_count == 2;

        vec![
            (
                BankMappingType::CpuRam,
                0,
                BankMapping {
                    ty: BankMappingType::CpuRam,
                    to: 0,
                    read: sram_count > 0,
                    write: sram_count > 0,
                },
            ),
            (
                BankMappingType::CpuRom,
                0,
                BankMapping {
                    ty: BankMappingType::CpuRom,
                    to: 0,
                    read: true,
                    write: false,
                },
            ),
            (
                BankMappingType::CpuRom,
                1,
                BankMapping {
                    ty: BankMappingType::CpuRom,
                    to: if has_32kb_prg_rom { 1 } else { 0 },
                    read: true,
                    write: false,
                },
            ),
            (
                BankMappingType::Ppu,
                0,
                BankMapping {
                    ty: BankMappingType::Ppu,
                    to: 0,
                    read: true,
                    write: is_chr_ram,
                },
            ),
        ]
    }

    fn cpu_ram_bank_size(&self) -> u16 {
        0x2000
    }

    fn cpu_rom_bank_size(&self) -> u16 {
        0x4000
    }

    fn ppu_bank_size(&self) -> u16 {
        0x2000
    }

    fn registers_memory_range(&self) -> std::ops::RangeInclusive<u16> {
        // empty bounds
        2..=1
    }

    fn write_register(&mut self, _address: u16, _data: u8) -> Vec<(u8, BankMapping)> {
        unreachable!()
    }
}
