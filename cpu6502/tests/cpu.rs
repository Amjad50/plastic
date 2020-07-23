extern crate cpu6502;

use common::{interconnection::*, Bus, Device};
use cpu6502::{CPURunState, CPU6502};
use std::{cell::RefCell, rc::Rc};

struct DummyBus {
    data: [u8; 0x10000],
}

impl DummyBus {
    pub fn new(data: [u8; 0x10000]) -> Self {
        Self { data }
    }
}

impl Bus for DummyBus {
    fn read(&self, address: u16, _: Device) -> u8 {
        self.data[address as usize]
    }
    fn write(&mut self, address: u16, data: u8, _: Device) {
        self.data[address as usize] = data;
    }
}

struct DummyCartridgePPUHandler {}

impl PPUCPUConnection for DummyCartridgePPUHandler {
    fn is_nmi_pin_set(&self) -> bool {
        false
    }
    fn clear_nmi_pin(&mut self) {}
    fn is_dma_request(&self) -> bool {
        false
    }
    fn clear_dma_request(&mut self) {}
    fn dma_address(&mut self) -> u8 {
        unreachable!()
    }
    fn send_oam_data(&mut self, _address: u8, _data: u8) {
        unreachable!();
    }
}

impl CartridgeCPUConnection for DummyCartridgePPUHandler {
    fn is_irq_requested(&self) -> bool {
        false
    }
    fn clear_irq_request_pin(&mut self) {}
}

#[test]
fn functionality_test() {
    let file_data = include_bytes!("./roms/6502_functional_test.bin");
    let mut data = [0; 0x10000];
    data[0xa..file_data.len() + 0xa].clone_from_slice(file_data);

    const SUCCUSS_ADDRESS: u16 = 0x336d;

    let bus = DummyBus::new(data);
    let handler = Rc::new(RefCell::new(DummyCartridgePPUHandler {}));
    let mut cpu = CPU6502::new(Rc::new(RefCell::new(bus)), handler.clone(), handler);

    cpu.reg_pc = 0x400;

    loop {
        let state = cpu.run_next();

        // if we stuck in a loop, return error
        if let CPURunState::InfiniteLoop(pc) = state {
            assert!(
                pc == SUCCUSS_ADDRESS,
                "Test failed at {:04X}, check the `.lst` file for more info",
                pc
            );
            break;
        }
    }
}
