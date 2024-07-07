use crate::common::{save_state::Savable, Bus, Device};
use crate::ppu2c02::PPU2C02;
use std::convert::TryInto;

memory_mapped_registers! {
    pub enum Register {
        Control = 0x2000,
        Mask = 0x2001,
        Status = 0x2002,
        OmaAddress = 0x2003,
        OmaData = 0x2004,
        Scroll = 0x2005,
        PPUAddress = 0x2006,
        PPUData = 0x2007,
        DmaOma = 0x4014,
    }
}

impl<T> Bus for PPU2C02<T>
where
    T: Bus + Savable,
{
    fn read(&self, address: u16, device: Device) -> u8 {
        // only the CPU is allowed to read from PPU registers
        if device == Device::Cpu {
            if let Ok(register) = address.try_into() {
                self.read_register(register)
            } else {
                unreachable!("Bus address mapping should be handled correctly (PPU Memory I/O)");
            }
        } else {
            unreachable!("CPU is the only device allowed to read from PPU registers");
        }
    }

    fn write(&mut self, address: u16, data: u8, device: Device) {
        // only the CPU is allowed to write to PPU registers
        if device == Device::Cpu {
            if let Ok(register) = address.try_into() {
                self.write_register(register, data);
            } else {
                unreachable!("Bus address mapping should be handled correctly (PPU Memory I/O)");
            }
        } else {
            unreachable!("CPU is the only device allowed to write to PPU registers");
        }
    }
}
