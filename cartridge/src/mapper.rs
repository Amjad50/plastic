use common::{Device, MirroringMode};

pub trait Mapper {
    fn init(
        &mut self,
        pgr_count: u8,
        is_chr_ram: bool,
        chr_count: u8,
        contain_sram: bool,
        sram_count: u8,
    );

    /// takes `address` to map from and `device`, then return
    /// (`allow_read`, `real_address`), where `real_address` is the address
    /// to read from in the data array stored by the cartidge class or any class
    /// using this trait
    ///
    /// in case of `allow_read` is false, `real_address` MUST be ignored as it
    /// has undefined address
    fn map_read(&self, address: u16, device: Device) -> (bool, usize);

    /// takes `address` to map from and `device`, then return
    /// (`allow_write`, `real_address`), where `real_address` is the address
    /// to write `data` to in the data array stored by the cartidge class or any class
    /// using this trait
    ///
    /// in case of `allow_write` is false, `real_address` MUST be ignored as it
    /// has undefined address
    fn map_write(&mut self, address: u16, data: u8, device: Device) -> (bool, usize);
    fn is_hardwired_mirrored(&self) -> bool;
    fn nametable_mirroring(&self) -> MirroringMode;
    fn is_irq_pin_state_changed_requested(&self) -> bool;
    fn irq_pin_state(&self) -> bool;
    fn clear_irq_request_pin(&mut self);
}
