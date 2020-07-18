use common::{Bus, Device, MirroringMode, MirroringProvider};
use std::{cell::RefCell, rc::Rc};

pub struct VRam {
    // half of this memory (or depending on mirroring mode) is unused
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

    /*
     * When mirroring, each segment is 0x3FF in size, 0x1000 / 4
     *
     * When mapping vertical and horizontal mirroring
     *
     * Vertical:   0x8 is equal to 0x0 => 0b1000 is equal 0b0000,
     *             0xc is equal to 0x4 => 0b1100 is equal 0b0100
     * we can achieve that by ANDing with 0x7 (0b0111) to remove the 4th bit
     *
     * Horizontal: 0x4 is equal to 0x0 => 0b0100 is equal 0b0000,
     *             0xc is equal to 0x8 => 0b1100 is equal 0b1000
     * we can achieve that by ANDing with 0xB (0b1011) to remove the 3rd bit
     */
    fn map_address(&self, address: u16) -> u16 {
        assert!(address >= 0x2000 && address < 0x3000);

        match self.mirroring_provider.borrow().mirroring_mode() {
            MirroringMode::Vertical => address & 0x7FF,
            MirroringMode::Horizontal => address & 0xBFF,
            _ => unimplemented!(
                "mirroring mode {:?}",
                self.mirroring_provider.borrow().mirroring_mode()
            ),
        }
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
