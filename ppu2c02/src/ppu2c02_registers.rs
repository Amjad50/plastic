use crate::ppu2c02::PPU2C02;

use common::{Bus, Device};

use std::convert::TryFrom;
use std::convert::TryInto;

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

impl TryFrom<u16> for Register {
    type Error = ();

    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            x if x == Register::Control as u16 => Ok(Register::Control),
            x if x == Register::Mask as u16 => Ok(Register::Mask),
            x if x == Register::Status as u16 => Ok(Register::Status),
            x if x == Register::OmaAddress as u16 => Ok(Register::OmaAddress),
            x if x == Register::OmaData as u16 => Ok(Register::OmaData),
            x if x == Register::Scroll as u16 => Ok(Register::Scroll),
            x if x == Register::PPUAddress as u16 => Ok(Register::PPUAddress),
            x if x == Register::PPUData as u16 => Ok(Register::PPUData),
            x if x == Register::DmaOma as u16 => Ok(Register::DmaOma),
            _ => Err(()),
        }
    }
}

impl<T> Bus for PPU2C02<T>
where
    T: Bus,
{
    fn read(&self, address: u16, device: Device) -> u8 {
        // only the CPU is allowed to read from PPU registers
        if device == Device::CPU {
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
        if device == Device::CPU {
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
