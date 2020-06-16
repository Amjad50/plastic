pub mod instruction;
mod cpu6502;

pub use crate::cpu6502::CPU6502;

// TODO: move outside
// this is just for testing
pub trait Bus {
    fn read(&self, address: u16) -> u8;
    fn get_pointer(&mut self, address: u16) -> &mut u8;
    fn write(&mut self, address: u16, data: u8);
}