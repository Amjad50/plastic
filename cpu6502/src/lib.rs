use common::interconnection::*;
use common::save_state::Savable;

mod cpu6502;
pub mod instruction;

mod tests;

pub use crate::cpu6502::CPURunState;
pub use crate::cpu6502::CPU6502;

pub trait CPUBusTrait: Savable + PPUCPUConnection + APUCPUConnection + CPUIrqProvider {
    fn read(&self, address: u16) -> u8;

    fn write(&mut self, address: u16, data: u8);

    fn reset(&mut self);
}
