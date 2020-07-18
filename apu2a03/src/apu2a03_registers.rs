use crate::apu2a03::APU2A03;
use common::{Bus, Device};
use std::convert::TryInto;

memory_mapped_registers! {
    pub enum Register {
        Pulse1_1 = 0x4000,
        Pulse1_2 = 0x4001,
        Pulse1_3 = 0x4002,
        Pulse1_4 = 0x4003,

        Pulse2_1 = 0x4004,
        Pulse2_2 = 0x4005,
        Pulse2_3 = 0x4006,
        Pulse2_4 = 0x4007,

        Triangle1 = 0x4008,
        Triangle2 = 0x4009, // unused
        Triangle3 = 0x400A,
        Triangle4 = 0x400B,

        Noise1 = 0x400C,
        Noise2 = 0x400D, // unused
        Noise3 = 0x400E,
        Noise4 = 0x400F,

        DMC1 = 0x4010,
        DMC2 = 0x4011,
        DMC3 = 0x4012,
        DMC4 = 0x4013,

        Status = 0x4015,

        FrameCounter = 0x4017,
    }
}

impl Bus for APU2A03 {
    fn read(&self, address: u16, device: Device) -> u8 {
        // only the CPU is allowed to read from PPU registers
        if device == Device::CPU {
            if let Ok(register) = address.try_into() {
                self.read_register(register)
            } else {
                unreachable!("Bus address mapping should be handled correctly (APU Memory I/O)");
            }
        } else {
            unreachable!("CPU is the only device allowed to read from APU registers");
        }
    }

    fn write(&mut self, address: u16, data: u8, device: Device) {
        // only the CPU is allowed to write to PPU registers
        if device == Device::CPU {
            if let Ok(register) = address.try_into() {
                self.write_register(register, data);
            } else {
                unreachable!("Bus address mapping should be handled correctly (APU Memory I/O)");
            }
        } else {
            unreachable!("CPU is the only device allowed to write to APU registers");
        }
    }
}
