use crate::apu2a03::APU2A03;
use crate::cartridge::{Cartridge, CartridgeError};
use crate::common::{
    interconnection::*,
    save_state::{Savable, SaveError},
    Bus, Device,
};
use crate::cpu6502::{CPUBusTrait, CPURunState, CPU6502};
use crate::display::{COLORS, TV};
use crate::ppu2c02::{Palette, VRam, PPU2C02};
use std::{
    cell::{Cell, RefCell},
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

impl PPUBus {
    pub fn new(cartridge: Rc<RefCell<Cartridge>>) -> Self {
        PPUBus {
            cartridge: cartridge.clone(),
            vram: VRam::new(cartridge),
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

impl Savable for PPUBus {
    fn save<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), SaveError> {
        unreachable!()
    }

    fn load<R: std::io::Read>(&mut self, _reader: &mut R) -> Result<(), SaveError> {
        unreachable!()
    }
}

struct CPUBus {
    cartridge: Rc<RefCell<Cartridge>>,
    ram: [u8; 0x800],
    ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
    apu: Rc<RefCell<APU2A03>>,
    irq_pin_change_requested: Cell<bool>,
}

impl CPUBus {
    pub fn new(
        cartridge: Rc<RefCell<Cartridge>>,
        ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
        apu: Rc<RefCell<APU2A03>>,
    ) -> Self {
        CPUBus {
            cartridge,
            ram: [0; 0x800],
            ppu,
            apu,
            irq_pin_change_requested: Cell::new(false),
        }
    }
}

impl CPUBusTrait for CPUBus {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize],
            0x2000..=0x3FFF => self
                .ppu
                .borrow()
                .read(0x2000 | (address & 0x7), Device::CPU),
            0x4000..=0x4013 => self.apu.borrow().read(address, Device::CPU),
            0x4014 => self.ppu.borrow().read(address, Device::CPU),
            0x4015 => self.apu.borrow().read(address, Device::CPU),
            0x4016 => {
                // controller
                0
            }
            0x4017 => self.apu.borrow().read(address, Device::CPU),
            0x4018..=0x401F => {
                // unused CPU test mode registers
                0
            }
            0x4020..=0xFFFF => self.cartridge.borrow().read(address, Device::CPU),
        }
    }
    fn write(&mut self, address: u16, data: u8) {
        match address {
            0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize] = data,
            0x2000..=0x3FFF => {
                self.ppu
                    .borrow_mut()
                    .write(0x2000 | (address & 0x7), data, Device::CPU)
            }
            0x4000..=0x4013 => self.apu.borrow_mut().write(address, data, Device::CPU),
            0x4014 => self.ppu.borrow_mut().write(address, data, Device::CPU),
            0x4015 => self.apu.borrow_mut().write(address, data, Device::CPU),
            0x4016 => {
                // controller
            }
            0x4017 => self.apu.borrow_mut().write(address, data, Device::CPU),
            0x4018..=0x401F => {
                // unused CPU test mode registers
            }
            0x4020..=0xFFFF => self
                .cartridge
                .borrow_mut()
                .write(address, data, Device::CPU),
        };
    }

    fn reset(&mut self) {
        self.ram = [0; 0x800];
    }
}

impl Savable for CPUBus {
    fn save<W: std::io::Write>(&self, _: &mut W) -> Result<(), SaveError> {
        unreachable!()
    }

    fn load<R: std::io::Read>(&mut self, _: &mut R) -> Result<(), SaveError> {
        unreachable!()
    }
}

impl PPUCPUConnection for CPUBus {
    fn is_nmi_pin_set(&self) -> bool {
        self.ppu.borrow().is_nmi_pin_set()
    }

    fn clear_nmi_pin(&mut self) {
        self.ppu.borrow_mut().clear_nmi_pin()
    }

    fn is_dma_request(&self) -> bool {
        self.ppu.borrow_mut().is_dma_request()
    }

    fn clear_dma_request(&mut self) {
        self.ppu.borrow_mut().clear_dma_request()
    }

    fn dma_address(&mut self) -> u8 {
        self.ppu.borrow_mut().dma_address()
    }

    fn send_oam_data(&mut self, address: u8, data: u8) {
        self.ppu.borrow_mut().send_oam_data(address, data)
    }
}

impl APUCPUConnection for CPUBus {
    fn request_dmc_reader_read(&self) -> Option<u16> {
        self.apu.borrow().request_dmc_reader_read()
    }

    fn submit_dmc_buffer_byte(&mut self, byte: u8) {
        self.apu.borrow_mut().submit_dmc_buffer_byte(byte)
    }
}

impl CPUIrqProvider for CPUBus {
    fn is_irq_change_requested(&self) -> bool {
        let result = self.apu.borrow().is_irq_change_requested()
            || self.cartridge.borrow().is_irq_change_requested();

        self.irq_pin_change_requested.set(result);
        result
    }

    fn irq_pin_state(&self) -> bool {
        if self.irq_pin_change_requested.get() {
            let mut result = self.apu.borrow().irq_pin_state();
            if self.cartridge.borrow().is_irq_change_requested() {
                result = result || self.cartridge.borrow().irq_pin_state();
            }
            result
        } else {
            false
        }
    }

    fn clear_irq_request_pin(&mut self) {
        *self.irq_pin_change_requested.get_mut() = false;
        self.cartridge.borrow_mut().clear_irq_request_pin();
        self.apu.borrow_mut().clear_irq_request_pin();
    }
}

pub struct NES {
    cpu: CPU6502<CPUBus>,
    ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
    tv_image: Arc<Mutex<Vec<u8>>>,
    apu: Rc<RefCell<APU2A03>>,
}

impl NES {
    pub fn new(filename: &str) -> Result<Self, CartridgeError> {
        let cartridge = Rc::new(RefCell::new(Cartridge::from_file(filename)?));

        let ppubus = PPUBus::new(cartridge.clone());

        let tv = TV::new(|color| [color.r, color.g, color.b, 0xFF]);
        let tv_image = tv.get_image_clone();

        let ppu = Rc::new(RefCell::new(PPU2C02::new(ppubus, tv)));

        let apu = Rc::new(RefCell::new(APU2A03::new()));

        let cpubus = CPUBus::new(cartridge, ppu.clone(), apu.clone());

        let cpu = CPU6502::new(cpubus);

        Ok(Self {
            cpu,
            ppu,
            tv_image,
            apu,
        })
    }

    pub fn reset_cpu(&mut self) {
        self.cpu.reset();
    }

    pub fn cpu_read_address(&self, address: u16) -> u8 {
        self.cpu.bus().read(address)
    }

    pub fn ppu_read_address(&self, address: u16) -> u8 {
        self.ppu.borrow().ppu_bus().read(address, Device::PPU)
    }

    pub fn clock(&mut self) -> CPURunState {
        self.apu.borrow_mut().clock();

        let return_value = self.cpu.run_next();

        {
            let mut ppu = self.ppu.borrow_mut();

            ppu.clock();
            ppu.clock();
            ppu.clock();
        }

        return_value
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

            if self.cpu.bus().read(address) != data {
                break;
            }
        }
    }

    /// after each CPU clock (3 PPU clocks), check if the pixel in `x, y`
    /// match the color specified `color_code`, if match, then return
    ///
    /// this check is done manually now, not sure if it should be added
    /// to `display::TV` or not
    #[allow(clippy::identity_op)]
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
