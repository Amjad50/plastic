#[cfg(test)]
mod ppu_tests {
    use crate::{Palette, VRam, PPU2C02};
    use cartridge::{Cartridge, CartridgeError};
    use common::{Bus, Device};
    use cpu6502::{CPURunState, CPU6502};
    use display::TV;
    use std::{
        cell::RefCell,
        convert::From,
        error::Error,
        fmt::{Debug, Display, Formatter, Result as fmtResult},
        fs::File,
        rc::Rc,
    };

    enum PPUTestError {
        CartridgeError(CartridgeError),
        ResultError(u8),
    }
    impl PPUTestError {
        fn get_message(&self) -> String {
            match self {
                Self::CartridgeError(err) => format!("CartridgeError: {}", err),
                Self::ResultError(code) => format!("ResultError: test failed with code {}", code),
            }
        }
    }

    impl Error for PPUTestError {}

    impl Display for PPUTestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
            write!(f, "{}", self.get_message())
        }
    }

    impl Debug for PPUTestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
            write!(f, "{}", self.get_message())
        }
    }

    impl From<CartridgeError> for PPUTestError {
        fn from(from: CartridgeError) -> Self {
            Self::CartridgeError(from)
        }
    }

    struct PPUBus {
        cartridge: Rc<RefCell<Cartridge>>,
        vram: VRam,
        palettes: Palette,
    }

    struct CPUBus {
        cartridge: Rc<RefCell<Cartridge>>,
        ram: [u8; 0x800],
        ppu: Rc<RefCell<dyn Bus>>,
    }

    impl CPUBus {
        pub fn new(cartridge: Rc<RefCell<Cartridge>>, ppu: Rc<RefCell<dyn Bus>>) -> Self {
            CPUBus {
                cartridge,
                ram: [0; 0x800],
                ppu,
            }
        }
    }

    impl PPUBus {
        pub fn new(cartridge: Rc<RefCell<Cartridge>>, is_vertical_mirroring: bool) -> Self {
            PPUBus {
                cartridge,
                vram: VRam::new(is_vertical_mirroring),
                palettes: Palette::new(),
            }
        }
    }

    impl Bus for PPUBus {
        fn read(&self, address: u16, device: Device) -> u8 {
            match address {
                0x0000..=0x1FFF => self.cartridge.borrow().read(address, device),
                0x2000..=0x3EFF => self.vram.read(address & 0x2FFF, device),
                0x3F00..=0x3FFF => self.palettes.read(address, device),
                // mirror
                0x4000..=0xFFFF => self.read(address & 0x3FFF, device),
            }
        }
        fn write(&mut self, address: u16, data: u8, device: Device) {
            match address {
                0x0000..=0x1FFF => self.cartridge.borrow_mut().write(address, data, device),
                0x2000..=0x3EFF => self.vram.write(address & 0x2FFF, data, device),
                0x3F00..=0x3FFF => self.palettes.write(address, data, device),
                // mirror
                0x4000..=0xFFFF => self.write(address & 0x3FFF, data, device),
            }
        }
    }

    impl Bus for CPUBus {
        fn read(&self, address: u16, device: Device) -> u8 {
            match address {
                0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize],
                0x2000..=0x3FFF => self.ppu.borrow().read(0x2000 | (address & 0x7), device),
                0x4014 => self.ppu.borrow().read(address, device),
                0x8000..=0xFFFF => self.cartridge.borrow().read(address, device),
                _ => {
                    // ignored
                    0
                }
            }
        }
        fn write(&mut self, address: u16, data: u8, device: Device) {
            match address {
                0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize] = data,
                0x2000..=0x3FFF => {
                    self.ppu
                        .borrow_mut()
                        .write(0x2000 | (address & 0x7), data, device)
                }
                0x4014 => self.ppu.borrow_mut().write(address, data, device),
                0x8000..=0xFFFF => self
                    .cartridge
                    .borrow_mut()
                    .write(address, data, Device::CPU),
                _ => {
                    // ignored
                }
            };
        }
    }

    struct NES {
        cpu: CPU6502<CPUBus>,
        ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
        cpubus: Rc<RefCell<CPUBus>>,
    }

    impl NES {
        fn new(filename: &str) -> Result<Self, CartridgeError> {
            let cartridge = Rc::new(RefCell::new(Cartridge::from_file(File::open(filename)?)?));

            let ppubus = PPUBus::new(
                cartridge.clone(),
                cartridge.borrow().is_vertical_mirroring(),
            );
            // FIXME: used constants hosted in TV
            const TV_WIDTH: u32 = 256;
            const TV_HEIGHT: u32 = 240;

            let tv = TV::new(TV_WIDTH, TV_HEIGHT);

            let ppu = Rc::new(RefCell::new(PPU2C02::new(ppubus, tv)));

            let cpubus = Rc::new(RefCell::new(CPUBus::new(cartridge.clone(), ppu.clone())));

            let cpu = CPU6502::new(cpubus.clone(), ppu.clone());

            Ok(Self {
                cpu,
                ppu: ppu.clone(),
                cpubus: cpubus.clone(),
            })
        }

        fn clock(&mut self) -> CPURunState {
            {
                let mut ppu = self.ppu.borrow_mut();

                ppu.run_cycle();
                ppu.run_cycle();
                ppu.run_cycle();
            }

            self.cpu.run_next()
        }

        fn clock_until_infinite_loop(&mut self) {
            loop {
                if let CPURunState::InfiniteLoop(_) = self.clock() {
                    break;
                }
            }
        }

        fn clock_until_nmi(&mut self) {
            loop {
                if let CPURunState::StartingInterrupt = self.clock() {
                    break;
                }
            }
        }
    }

    fn run_test(filename: &str, result_memory_address: u16) -> Result<(), PPUTestError> {
        let mut nes = NES::new(filename)?;
        nes.cpu.reset();

        nes.clock_until_infinite_loop();

        let result = nes.cpubus.borrow().read(result_memory_address, Device::CPU);

        if result != 1 {
            Err(PPUTestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    #[test]
    fn blargg_ppu_test_palette_ram() -> Result<(), PPUTestError> {
        run_test("./tests/roms/blargg_ppu_tests/palette_ram.nes", 0x00f0)
    }

    #[test]
    fn blargg_ppu_test_power_up_palette() -> Result<(), PPUTestError> {
        run_test("./tests/roms/blargg_ppu_tests/power_up_palette.nes", 0x00f0)
    }

    #[test]
    fn blargg_ppu_test_sprite_ram() -> Result<(), PPUTestError> {
        run_test("./tests/roms/blargg_ppu_tests/sprite_ram.nes", 0x00f0)
    }

    // FIXME: this test is still failing
    // #[test]
    fn blargg_ppu_test_vbl_clear_time() -> Result<(), PPUTestError> {
        let filename = "./tests/roms/blargg_ppu_tests/vbl_clear_time.nes";
        let result_memory_address = 0x00f0;

        let mut nes = NES::new(filename)?;
        nes.cpu.reset();

        // 2 NMIs should occure
        nes.clock_until_nmi();
        nes.clock_until_nmi();
        nes.clock_until_infinite_loop();

        let result = nes.cpubus.borrow().read(result_memory_address, Device::CPU);

        if result != 1 {
            Err(PPUTestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    #[test]
    fn blargg_ppu_test_vram_access() -> Result<(), PPUTestError> {
        run_test("./tests/roms/blargg_ppu_tests/vram_access.nes", 0x00f0)
    }
}
