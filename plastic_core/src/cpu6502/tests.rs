#[cfg(test)]
mod cpu_tests {
    use super::super::{CPUBusTrait, CPURunState, CPU6502};
    use crate::common::{interconnection::*, save_state::Savable};

    struct DummyBus {
        data: [u8; 0x10000],
    }

    impl DummyBus {
        pub fn new(data: [u8; 0x10000]) -> Self {
            Self { data }
        }
    }

    impl Savable for DummyBus {
        fn save<W: std::io::Write>(
            &self,
            _: &mut W,
        ) -> Result<(), crate::common::save_state::SaveError> {
            unreachable!()
        }

        fn load<R: std::io::Read>(
            &mut self,
            _: &mut R,
        ) -> Result<(), crate::common::save_state::SaveError> {
            unreachable!()
        }
    }

    impl CPUBusTrait for DummyBus {
        fn read(&self, address: u16) -> u8 {
            self.data[address as usize]
        }
        fn write(&mut self, address: u16, data: u8) {
            self.data[address as usize] = data;
        }

        fn reset(&mut self) {
            unreachable!()
        }
    }

    impl PPUCPUConnection for DummyBus {
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

    impl APUCPUConnection for DummyBus {
        fn request_dmc_reader_read(&self) -> Option<u16> {
            None
        }
        fn submit_dmc_buffer_byte(&mut self, _: u8) {
            unreachable!();
        }
    }

    impl CPUIrqProvider for DummyBus {
        fn is_irq_change_requested(&self) -> bool {
            false
        }

        fn irq_pin_state(&self) -> bool {
            unreachable!();
        }

        fn clear_irq_request_pin(&mut self) {
            unreachable!();
        }
    }

    #[test]
    fn functionality_test() {
        let file_data =
            include_bytes!("../../../test_roms/6502_functional_test/6502_functional_test.bin");
        let mut data = [0; 0x10000];
        data[0xa..file_data.len() + 0xa].clone_from_slice(file_data);

        // set the reset vector pointer to 0x0400
        data[0xFFFC] = 0x00;
        data[0xFFFD] = 0x04;

        const SUCCUSS_ADDRESS: u16 = 0x336D;

        let bus = DummyBus::new(data);
        let mut cpu = CPU6502::new(bus);

        cpu.reset();

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
}
