use cartridge::{Cartridge, CartridgeError};
use common::{Bus, Device};
use cpu6502::CPU6502;
use display::TV;
use ppu2c02::PPU2C02;
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
    vram: [u8; 0x1000],
    palettes: [u8; 0x20],
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
    pub fn new(cartridge: Rc<RefCell<Cartridge>>) -> Self {
        PPUBus {
            cartridge,
            vram: [0; 0x1000],
            palettes: [0; 0x20],
        }
    }
}

impl Bus for PPUBus {
    fn read(&self, address: u16, device: Device) -> u8 {
        match address {
            0x0000..=0x1FFF => self.cartridge.borrow().read(address, device),
            0x2000..=0x3EFF => self.vram[(address & 0xFFF) as usize],
            0x3F00..=0x3FFF => self.palettes[(address & 0x1F) as usize],
            // mirror
            0x4000..=0xFFFF => self.read(address & 0x3FFF, device),
        }
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        match address {
            0x0000..=0x1FFF => self.cartridge.borrow_mut().write(address, data, device),
            0x2000..=0x3EFF => self.vram[(address & 0xFFF) as usize] = data,
            0x3F00..=0x3FFF => self.palettes[(address & 0x1F) as usize] = data,
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
            0x4014 => self.ppu.borrow_mut().write(address, data, device),
            0x8000..=0xFFFF => self
                .cartridge
                .borrow_mut()
                .write(address, data, Device::CPU),
            _ => println!("unimplemented write cpu to {:04X}", address),
        };
    }
}

pub struct NES {
    cpu: CPU6502<CPUBus>,
    ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
    image: Arc<Mutex<Vec<u8>>>,
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

        let cpubus = CPUBus::new(cartridge.clone(), ppu.clone());

        let cpu = CPU6502::new(cpubus, ppu.clone());

        Ok(Self {
            cpu,
            ppu: ppu.clone(),
            image,
        })
    }

    pub fn run(&mut self) {
        self.cpu.reset();

        // channel for sending a stop signal for cpu/ppu clock
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<bool>();

        let image = self.image.clone();

        let thread = std::thread::spawn(move || {
            let mut window = RenderWindow::new(
                (SCREEN_WIDTH, SCREEN_HEIGHT),
                "NES test",
                Style::CLOSE,
                &Default::default(),
            );
            window.set_vertical_sync_enabled(true);

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
                // TODO: also handle NES controller input here later
                while let Some(event) = window.poll_event() {
                    match event {
                        Event::Closed
                        | Event::KeyPressed {
                            code: Key::Escape, ..
                        } => break 'main,
                        _ => {}
                    }
                }

                window.clear(Color::BLACK);

                let pixels = &image.lock().unwrap();

                let image = Image::create_from_pixels(TV_WIDTH, TV_HEIGHT, pixels).expect("image");

                texture.update_from_image(&image, 0, 0);

                window.draw(&Sprite::with_texture(&texture));

                window.display();
            }
            // when the window is stopped, stop the ppu/cpu clock
            stop_tx.send(true).unwrap();
        });

        loop {
            self.cpu.run_next();

            let mut ppu = self.ppu.borrow_mut();
            ppu.run_cycle();
            ppu.run_cycle();
            ppu.run_cycle();
            if let Ok(value) = stop_rx.recv_timeout(std::time::Duration::from_nanos(1)) {
                if value {
                    break;
                }
            }
        }
        thread.join().unwrap();
    }
}
