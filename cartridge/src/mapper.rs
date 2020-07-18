use common::{Device, MirroringMode};

pub trait Mapper {
    fn init(&mut self, pgr_count: u8, chr_count: u8);
    fn map_read(&self, address: u16, device: Device) -> usize;
    fn map_write(&mut self, address: u16, data: u8, device: Device);
    fn is_hardwired_mirrored(&self) -> bool;
    fn nametable_mirroring(&self) -> MirroringMode;
}
