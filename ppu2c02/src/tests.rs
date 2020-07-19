#[cfg(test)]
mod ppu_tests {
    use crate::{Palette, VRam, PPU2C02};
    use cartridge::{Cartridge, CartridgeError};
    use common::{Bus, Device};
    use cpu6502::{CPURunState, CPU6502};
    use display::{COLORS, TV};
    use std::{
        cell::RefCell,
        convert::From,
        error::Error,
        fmt::{Debug, Display, Formatter, Result as fmtResult},
        fs::File,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    // FIXME: used constants hosted in TV
    const TV_WIDTH: u32 = 256;
    const TV_HEIGHT: u32 = 240;

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
        battery_ram: [u8; 0x2000],
        ppu: Rc<RefCell<dyn Bus>>,
    }

    impl CPUBus {
        pub fn new(cartridge: Rc<RefCell<Cartridge>>, ppu: Rc<RefCell<dyn Bus>>) -> Self {
            CPUBus {
                cartridge,
                ram: [0; 0x800],
                battery_ram: [0; 0x2000],
                ppu,
            }
        }
    }

    impl PPUBus {
        pub fn new(cartridge: Rc<RefCell<Cartridge>>) -> Self {
            PPUBus {
                cartridge: cartridge.clone(),
                vram: VRam::new(cartridge.clone()),
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
                0x6000..=0x7FFF => self.battery_ram[(address & 0x1FFF) as usize],
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
                0x6000..=0x7FFF => self.battery_ram[(address & 0x1FFF) as usize] = data,
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
        tv_image: Arc<Mutex<Vec<u8>>>,
    }

    impl NES {
        fn new(filename: &str) -> Result<Self, CartridgeError> {
            let cartridge = Rc::new(RefCell::new(Cartridge::from_file(File::open(filename)?)?));

            let ppubus = PPUBus::new(cartridge.clone());

            let tv = TV::new(TV_WIDTH, TV_HEIGHT);
            let tv_image = tv.get_image_clone();

            let ppu = Rc::new(RefCell::new(PPU2C02::new(ppubus, tv)));

            let cpubus = Rc::new(RefCell::new(CPUBus::new(cartridge.clone(), ppu.clone())));

            let cpu = CPU6502::new(cpubus.clone(), ppu.clone());

            Ok(Self {
                cpu,
                ppu: ppu.clone(),
                cpubus: cpubus.clone(),
                tv_image,
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

        /// loop until the memory at `address` does not equal to `data`
        fn clock_until_memory_neq(&mut self, address: u16, data: u8) {
            loop {
                self.clock();

                if self.cpubus.borrow().read(address, Device::CPU) != data {
                    break;
                }
            }
        }

        /// after each CPU clock (3 PPU clocks), check if the pixel in `x, y`
        /// match the color specified `color_code`, if match, then return
        ///
        /// this check is done manually now, not sure if it should be added
        /// to `display::TV` or not
        fn clock_until_pixel_appears(&mut self, x: u32, y: u32, color_code: u8) {
            loop {
                self.clock();

                let index = (y * TV_WIDTH + x) as usize * 4;

                if let Ok(image) = self.tv_image.lock() {
                    let color = &COLORS[color_code as usize];

                    if image[index + 0] == color.r
                        && image[index + 1] == color.g
                        && image[index + 2] == color.b
                        && image[index + 3] == 0xFF
                    {
                        break;
                    }
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

    fn run_sprite_hit_test(filename: &str) -> Result<(), PPUTestError> {
        let result_memory_address = 0x00F8;

        let mut nes = NES::new(filename)?;
        nes.cpu.reset();

        // this is the top-left pixel of the word "PASSED" or "FAILED"
        nes.clock_until_pixel_appears(17, 48, 0x30);

        let result = nes.cpubus.borrow().read(result_memory_address, Device::CPU);

        if result != 1 {
            Err(PPUTestError::ResultError(result))
        } else {
            Ok(())
        }
    }

    fn run_ppu_vbl_nmi_test(filename: &str) -> Result<(), PPUTestError> {
        let result_memory_address = 0x6000;

        let mut nes = NES::new(filename)?;
        nes.cpu.reset();

        // first loop until an infnite loop (this infinite loop might be the
        // end or not), then loop until the value of `0x6000` is not `0x80`
        // the reason we can't loop until memory_neq from the beginning
        // is because the ram starts with all zeros, so it will stop after the
        // first instruction
        nes.clock_until_infinite_loop();
        // the default is 0x80, when the rom starts
        nes.clock_until_memory_neq(0x6000, 0x80);

        let result = nes.cpubus.borrow().read(result_memory_address, Device::CPU);

        if result != 0 {
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

    #[test]
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

    #[test]
    fn sprite_hit_test_01_basics() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/01.basics.nes")
    }

    #[test]
    fn sprite_hit_test_02_alignment() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/02.alignment.nes")
    }

    #[test]
    fn sprite_hit_test_03_corners() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/03.corners.nes")
    }

    #[test]
    fn sprite_hit_test_04_flip() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/04.flip.nes")
    }

    #[test]
    fn sprite_hit_test_05_left_clip() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/05.left_clip.nes")
    }

    #[test]
    fn sprite_hit_test_06_right_edge() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/06.right_edge.nes")
    }

    #[test]
    fn sprite_hit_test_07_screen_bottom() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/07.screen_bottom.nes")
    }

    #[test]
    fn sprite_hit_test_08_double_height() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/08.double_height.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn sprite_hit_test_09_timing_basics() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/09.timing_basics.nes")
    }

    #[test]
    fn sprite_hit_test_10_timing_order() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/10.timing_order.nes")
    }

    #[test]
    fn sprite_hit_test_11_edge_timing() -> Result<(), PPUTestError> {
        run_sprite_hit_test("./tests/roms/sprite_hit_tests/11.edge_timing.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_01_vbl_basics() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_02_vbl_set_time() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_03_vbl_clear_time() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_04_nmi_control() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/04-nmi_control.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_05_nmi_timing() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/05-nmi_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_06_suppression() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/06-suppression.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_07_nmi_on_timing() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_08_nmi_off_timing() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes")
    }

    #[test]
    fn ppu_vbl_nmi_test_09_even_odd_frames() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes")
    }

    // FIXME: this test is still failing
    // #[test]
    fn ppu_vbl_nmi_test_10_even_odd_timing() -> Result<(), PPUTestError> {
        run_ppu_vbl_nmi_test("./tests/roms/ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes")
    }
}
