#[derive(PartialEq, Clone, Copy)]
pub enum Device {
    CPU,
    PPU,
}

pub trait Bus {
    fn read(&self, address: u16, device: Device) -> u8;
    fn write(&mut self, address: u16, data: u8, device: Device);
}
