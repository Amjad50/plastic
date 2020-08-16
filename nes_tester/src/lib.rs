use apu2a03::APU2A03;
use cartridge::{Cartridge, CartridgeError};
use common::{Bus, Device};
use cpu6502::{CPURunState, CPU6502};
use display::{COLORS, TV};
use ppu2c02::{Palette, VRam, PPU2C02};
use std::{
    cell::RefCell,
    convert::From,
    error::Error,
    fmt::{Debug, Display, Formatter, Result as fmtResult},
    rc::Rc,
    sync::{Arc, Mutex},
};

// FIXME: used constants hosted in TV
const TV_WIDTH: u32 = 256;
const TV_HEIGHT: u32 = 240;

pub enum TestError {
    CartridgeError(CartridgeError),
    ResultError(u8),
}
impl TestError {
    fn get_message(&self) -> String {
        match self {
            Self::CartridgeError(err) => format!("CartridgeError: {}", err),
            Self::ResultError(code) => format!("ResultError: test failed with code {}", code),
        }
    }
}

impl Error for TestError {}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl Debug for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl From<CartridgeError> for TestError {
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
    apu: Rc<RefCell<APU2A03>>,
}

impl CPUBus {
    pub fn new(
        cartridge: Rc<RefCell<Cartridge>>,
        ppu: Rc<RefCell<dyn Bus>>,
        apu: Rc<RefCell<APU2A03>>,
    ) -> Self {
        CPUBus {
            cartridge,
            ram: [0; 0x800],
            ppu,
            apu,
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
            0x4000..=0x4013 => self.apu.borrow().read(address, device),
            0x4014 => self.ppu.borrow().read(address, device),
            0x4015 => self.apu.borrow().read(address, device),
            0x4017 => self.apu.borrow().read(address, device),
            0x6000..=0xFFFF => self.cartridge.borrow().read(address, device),
            _ => {
                // ignored
                0
            }
        }
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        match address {
            0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize] = data,
            0x2000..=0x3FFF => self
                .ppu
                .borrow_mut()
                .write(0x2000 | (address & 0x7), data, device),
            0x4000..=0x4013 => self.apu.borrow_mut().write(address, data, device),
            0x4014 => self.ppu.borrow_mut().write(address, data, device),
            0x4015 => self.apu.borrow_mut().write(address, data, device),
            0x4017 => self.apu.borrow_mut().write(address, data, device),
            0x6000..=0xFFFF => self
                .cartridge
                .borrow_mut()
                .write(address, data, Device::CPU),
            _ => {
                // ignored
            }
        };
    }
}

pub struct NES {
    cpu: CPU6502<CPUBus>,
    ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
    cpubus: Rc<RefCell<CPUBus>>,
    tv_image: Arc<Mutex<Vec<u8>>>,
    apu: Rc<RefCell<APU2A03>>,

    is_apu_clock: bool,
}

impl NES {
    pub fn new(filename: &str) -> Result<Self, CartridgeError> {
        let cartridge = Rc::new(RefCell::new(Cartridge::from_file(filename)?));

        let ppubus = PPUBus::new(cartridge.clone());

        let tv = TV::new(TV_WIDTH, TV_HEIGHT, |color| {
            [color.r, color.g, color.b, 0xFF]
        });
        let tv_image = tv.get_image_clone();

        let ppu = Rc::new(RefCell::new(PPU2C02::new(ppubus, tv)));

        let apu = Rc::new(RefCell::new(APU2A03::new()));

        let cpubus = Rc::new(RefCell::new(CPUBus::new(
            cartridge.clone(),
            ppu.clone(),
            apu.clone(),
        )));

        let mut cpu = CPU6502::new(cpubus.clone(), ppu.clone(), apu.clone());
        cpu.add_irq_provider(cartridge.clone());
        cpu.add_irq_provider(apu.clone());

        Ok(Self {
            cpu,
            ppu: ppu.clone(),
            cpubus: cpubus.clone(),
            tv_image,
            apu,

            is_apu_clock: false,
        })
    }

    pub fn reset_cpu(&mut self) {
        self.cpu.reset();
    }

    pub fn cpu_read_address(&self, address: u16) -> u8 {
        self.cpubus.borrow().read(address, Device::CPU)
    }

    pub fn ppu_read_address(&self, address: u16) -> u8 {
        self.ppu.borrow().ppu_bus().read(address, Device::PPU)
    }

    pub fn clock(&mut self) -> CPURunState {
        {
            let mut ppu = self.ppu.borrow_mut();

            ppu.clock();
            ppu.clock();
            ppu.clock();
        }

        if self.is_apu_clock {
            self.apu.borrow_mut().clock();
        }
        self.is_apu_clock = !self.is_apu_clock;

        self.cpu.run_next()
    }

    pub fn clock_until_infinite_loop(&mut self) {
        loop {
            if let CPURunState::InfiniteLoop(_) = self.clock() {
                break;
            }
        }
    }

    pub fn clock_until_nmi(&mut self) {
        loop {
            if let CPURunState::StartingInterrupt = self.clock() {
                break;
            }
        }
    }

    /// loop until the memory at `address` does not equal to `data`
    pub fn clock_until_memory_neq(&mut self, address: u16, data: u8) {
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
    pub fn clock_until_pixel_appears(&mut self, x: u32, y: u32, color_code: u8) {
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
