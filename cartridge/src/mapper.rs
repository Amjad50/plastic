use common::{Device, MirroringMode};

pub enum MappingResult {
    Allowed(usize),
    Denied,
}

pub trait Mapper {
    fn init(
        &mut self,
        pgr_count: u8,
        is_chr_ram: bool,
        chr_count: u8,
        contain_sram: bool,
        sram_count: u8,
    );

    /// takes `address` to map from and `device`, then return `result`
    /// if `result` is `MappingResult::Allowed`, then the `real_address` is
    /// the `usize` value, but if `result` is `MappingResult::Denied`, then there
    /// is no address to read from
    fn map_read(&self, address: u16, device: Device) -> MappingResult;

    /// takes `address` to map from and `device`, then return `result`
    /// if `result` is `MappingResult::Allowed`, then the `real_address` is
    /// the `usize` value, but if `result` is `MappingResult::Denied`, then there
    /// is no address to write to
    fn map_write(&mut self, address: u16, data: u8, device: Device) -> MappingResult;
    fn is_hardwired_mirrored(&self) -> bool;
    fn nametable_mirroring(&self) -> MirroringMode;
    fn is_irq_pin_state_changed_requested(&self) -> bool;
    fn irq_pin_state(&self) -> bool;
    fn clear_irq_request_pin(&mut self);
}
