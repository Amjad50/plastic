extern crate cpu6502;

use common::{Bus, Device};
use cpu6502::CPU6502;

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

#[test]
fn functionality_test() {
    let file_data = include_bytes!("./roms/6502_functional_test.bin");
    let mut data = [0; 0x10000];
    data[0xa..file_data.len() + 0xa].clone_from_slice(file_data);

    const SUCCUSS_ADDRESS: u16 = 0x336d;

    let bus = DummyBus::new(data);
    let mut cpu = CPU6502::new(bus);

    cpu.reg_pc = 0x400;

    let result = cpu.run_all();
    assert!(result.is_err());
    assert!(
        result.err().unwrap() == SUCCUSS_ADDRESS,
        "Test failed at {:04X}, check the `.lst` file for more info",
        result.err().unwrap()
    );
}
