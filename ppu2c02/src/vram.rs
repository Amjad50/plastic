use common::{Bus, Device, MirroringMode, MirroringProvider};
use std::{cell::RefCell, rc::Rc};

pub struct VRam {
    /// this have 4 blocks, only the first 2 are used for `Vertical`, `Horizontal`,
    /// and `SingleScreen` mirroring modes. The remaining 2 blocks are used for
    /// `FourScreen` mode
    vram_data: [u8; 0x1000],
    mirroring_provider: Rc<RefCell<dyn MirroringProvider>>,
}

impl VRam {
    pub fn new(mirroring_provider: Rc<RefCell<dyn MirroringProvider>>) -> Self {
        Self {
            vram_data: [0; 0x1000],
            mirroring_provider,
        }
    }

    fn map_address(&self, address: u16) -> usize {
        let block_num = match self.mirroring_provider.borrow().mirroring_mode() {
            MirroringMode::Vertical => (address >> 10) & 1,
            MirroringMode::Horizontal => (address >> 11) & 1,
            MirroringMode::SingleScreenLowBank => 0,
            MirroringMode::SingleScreenHighBank => 1,
            MirroringMode::FourScreen => {
                // directly return the address, as there is no mirroring, and
                // all the vram address is being used
                return address as usize & 0xFFF;
            }
        } as usize;

        let start_address = block_num << 10;

        start_address + (address as usize & 0x3FF)
    }
}

impl Bus for VRam {
    fn read(&self, address: u16, device: Device) -> u8 {
        assert!(device == Device::PPU);

        let address = self.map_address(address);

        self.vram_data[address as usize]
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        assert!(device == Device::PPU);

        let address = self.map_address(address);

        self.vram_data[address as usize] = data;
    }
}
