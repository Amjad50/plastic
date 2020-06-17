extern crate cpu6502;

use cpu6502::{Bus, CPU6502};

struct DummyBus {
    data: [u8; 0x10000],
}

impl DummyBus {
    pub fn new(data: [u8; 0x10000]) -> Self {
        Self { data: data }
    }
}

impl Bus for DummyBus {
    fn read(&self, address: u16) -> u8 {
        self.data[address as usize]
    }
    fn get_pointer(&mut self, address: u16) -> &mut u8 {
        &mut self.data[address as usize]
    }
    fn write(&mut self, address: u16, data: u8) {
        self.data[address as usize] = data;
    }
}

#[test]
fn functionality_test() {
    let file_data = *include_bytes!("./roms/6502_functional_test.bin");
    let mut data = [0; 0x10000];
    data[0xa..file_data.len() + 0xa].clone_from_slice(&file_data);

    let mut bus = DummyBus::new(data);
    let mut cpu = CPU6502::new(&mut bus);

    cpu.reg_pc = 0x400;

    cpu.run();
}
