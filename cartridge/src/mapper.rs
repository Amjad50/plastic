use common::MirroringMode;
use std::ops::RangeInclusive;

#[derive(Clone, Copy)]
pub enum BankMappingType {
    CpuRam,
    CpuRom,
    Ppu,
}

#[derive(Clone, Copy)]
pub struct BankMapping {
    /// type of the mapping (type is reserved, ty is being used)
    pub ty: BankMappingType,
    /// the bank in the whole memory to map to
    pub to: u16,
    /// allow read
    pub read: bool,
    /// allow write
    pub write: bool,
}

pub trait Mapper {
    /// initialize the mapper, should be called first.
    /// Returnes the initial bank mappings, should include ALL banks
    fn init(
        &mut self,
        pgr_count: u8,
        is_chr_ram: bool,
        chr_count: u8,
        sram_count: u8,
    ) -> Vec<(BankMappingType, u8, BankMapping)>;

    /// return the bank size in (bytes units) that should be used
    fn cpu_ram_bank_size(&self) -> u16;

    /// return the bank size in (bytes units) that should be used
    fn cpu_rom_bank_size(&self) -> u16;

    /// return the bank size (in bytes units) that should be used
    fn ppu_bank_size(&self) -> u16;

    /// the range of the CPU which are registers of this mapper
    fn registers_memory_range(&self) -> RangeInclusive<u16>;

    /// write to the register and get where it changed, returns a set of banks
    /// that should be changed
    fn write_register(&mut self, address: u16, data: u8) -> Vec<(u8, BankMapping)>;

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
