use apu2a03::APU2A03;
use cartridge::{Cartridge, CartridgeError};
use common::{Bus, Device};
use controller::{Controller, StandardNESControllerState, StandardNESKey};
use cpu6502::CPU6502;
use display::TV;
use ppu2c02::{Palette, VRam, PPU2C02};
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use sfml::{
    graphics::{Color, Image, RenderTarget, RenderWindow, Sprite, Texture, View},
    system::Vector2f,
    window::{Event, Key, Style},
};

// NES TV size
// TODO: should be included in "tv" crate
const TV_WIDTH: u32 = 256;
const TV_HEIGHT: u32 = 240;

const SCREEN_WIDTH: u32 = TV_WIDTH * 3;
const SCREEN_HEIGHT: u32 = TV_HEIGHT * 3;

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
    apu: Rc<RefCell<dyn Bus>>,
    contoller: Controller,
}

impl CPUBus {
    pub fn new(
        cartridge: Rc<RefCell<Cartridge>>,
        ppu: Rc<RefCell<dyn Bus>>,
        apu: Rc<RefCell<dyn Bus>>,
        contoller: Controller,
    ) -> Self {
        CPUBus {
            cartridge,
            ram: [0; 0x800],
            battery_ram: [0; 0x2000],
            ppu,
            apu,
            contoller,
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
            0x4016 => self.contoller.read(address, device),
            0x4017 => self.apu.borrow().read(address, device),
            0x6000..=0x7FFF => self.battery_ram[(address & 0x1FFF) as usize],
            0x8000..=0xFFFF => self.cartridge.borrow().read(address, device),
            _ => {
                println!("unimplemented read cpu from {:04X}", address);
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
            0x4016 => self.contoller.write(address, data, device),
            0x4017 => self.apu.borrow_mut().write(address, data, device),
            0x6000..=0x7FFF => self.battery_ram[(address & 0x1FFF) as usize] = data,
            0x8000..=0xFFFF => self
                .cartridge
                .borrow_mut()
                .write(address, data, Device::CPU),
            0x4016 => self.contoller.write(address, data, device),
            _ => println!("unimplemented write cpu to {:04X}", address),
        };
    }
}

pub struct NES {
    cpu: CPU6502<CPUBus>,
    ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
    apu: Rc<RefCell<APU2A03>>,
    image: Arc<Mutex<Vec<u8>>>,
    ctrl_state: Arc<Mutex<StandardNESControllerState>>,
}

impl NES {
    pub fn new(filename: &str) -> Result<Self, CartridgeError> {
        let cartridge = Cartridge::from_file(File::open(filename)?)?;
        let cartridge = Rc::new(RefCell::new(cartridge));
        let ppubus = PPUBus::new(cartridge.clone());

        let tv = TV::new(TV_WIDTH, TV_HEIGHT);
        let image = tv.get_image_clone();

        let ppu = PPU2C02::new(ppubus, tv);

        let ppu = Rc::new(RefCell::new(ppu));

        let apu = Rc::new(RefCell::new(APU2A03::new()));

        let ctrl = Controller::new();
        let ctrl_state = ctrl.get_primary_controller_state();

        let cpubus = CPUBus::new(cartridge.clone(), ppu.clone(), apu.clone(), ctrl);

        let cpu = CPU6502::new(Rc::new(RefCell::new(cpubus)), ppu.clone());

        Ok(Self {
            cpu,
            ppu,
            apu,
            image,
            ctrl_state,
        })
    }

    pub fn run(&mut self) {
        self.cpu.reset();
        // Run the sound thread
        self.apu.borrow().play();

        let image = self.image.clone();
        let ctrl_state = self.ctrl_state.clone();

        let mut window = RenderWindow::new(
            (SCREEN_WIDTH, SCREEN_HEIGHT),
            "NES test",
            Style::CLOSE,
            &Default::default(),
        );
        window.set_vertical_sync_enabled(true);
        window.set_framerate_limit(60);

        // to scale the view into the window
        // this view is in the size of the NES TV
        // but we can scale the window and all the pixels will be scaled
        // accordingly
        let view = View::new(
            Vector2f::new((TV_WIDTH / 2) as f32, (TV_HEIGHT / 2) as f32),
            Vector2f::new((TV_WIDTH) as f32, (TV_HEIGHT) as f32),
        );
        window.set_view(&view);

        let mut texture = Texture::new(TV_WIDTH, TV_HEIGHT).expect("texture");

        'main: loop {
            if let Ok(mut ctrl) = ctrl_state.lock() {
                while let Some(event) = window.poll_event() {
                    match event {
                        Event::Closed => break 'main,
                        Event::KeyPressed { code: key, .. } => match key {
                            Key::J => ctrl.press(StandardNESKey::B),
                            Key::K => ctrl.press(StandardNESKey::A),
                            Key::U => ctrl.press(StandardNESKey::Select),
                            Key::I => ctrl.press(StandardNESKey::Start),
                            Key::W => ctrl.press(StandardNESKey::Up),
                            Key::S => ctrl.press(StandardNESKey::Down),
                            Key::A => ctrl.press(StandardNESKey::Left),
                            Key::D => ctrl.press(StandardNESKey::Right),
                            _ => {}
                        },
                        Event::KeyReleased { code: key, .. } => match key {
                            Key::J => ctrl.release(StandardNESKey::B),
                            Key::K => ctrl.release(StandardNESKey::A),
                            Key::U => ctrl.release(StandardNESKey::Select),
                            Key::I => ctrl.release(StandardNESKey::Start),
                            Key::W => ctrl.release(StandardNESKey::Up),
                            Key::S => ctrl.release(StandardNESKey::Down),
                            Key::A => ctrl.release(StandardNESKey::Left),
                            Key::D => ctrl.release(StandardNESKey::Right),
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }

            // here we run the emulation in the GUI thread because we can
            // make SFML run `window.display` in 60 FPS, NTSC also run in 60FPS (almost)
            // so we can regulat the clock speed to match that, we know that 29780.5
            // cpu cycles happen in every frame
            const N: usize = 29780;
            // run the emulator loop
            for _ in 0..N {
                self.cpu.run_next();

                let mut ppu = self.ppu.borrow_mut();
                ppu.run_cycle();
                ppu.run_cycle();
                ppu.run_cycle();
            }

            window.clear(Color::BLACK);

            let pixels = &image.lock().unwrap();

            let image = Image::create_from_pixels(TV_WIDTH, TV_HEIGHT, pixels).expect("image");

            texture.update_from_image(&image, 0, 0);

            window.draw(&Sprite::with_texture(&texture));

            window.display();
        }
    }
}
