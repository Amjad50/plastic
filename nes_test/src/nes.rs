use apu2a03::APU2A03;
use cartridge::{Cartridge, CartridgeError};
use common::{Bus, Device};
use controller::{Controller, StandardNESControllerState, StandardNESKey};
use cpu6502::CPU6502;
use display::TV;
use ppu2c02::{Palette, VRam, PPU2C02};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use sfml::{
    graphics::{Color, FloatRect, Image, RenderTarget, RenderWindow, Sprite, Texture, View},
    system::{SfBox, Vector2f},
    window::{joystick::Axis, Event, Key, Style},
};

// NES TV size
// TODO: should be included in "tv" crate
const TV_WIDTH: u32 = 256;
const TV_HEIGHT: u32 = 240;

const SCREEN_SIZE_INCREASE: u32 = 3;

struct PPUBus {
    cartridge: Rc<RefCell<Cartridge>>,
    vram: VRam,
    palettes: Palette,
}

struct CPUBus {
    cartridge: Rc<RefCell<Cartridge>>,
    ram: [u8; 0x800],
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
            0x6000..=0xFFFF => self.cartridge.borrow().read(address, device),
            _ => {
                // println!("unimplemented read cpu from {:04X}", address);
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
            0x6000..=0xFFFF => self
                .cartridge
                .borrow_mut()
                .write(address, data, Device::CPU),
            _ => {} // println!("unimplemented write cpu to {:04X}", address),
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
        let cartridge = Cartridge::from_file(filename)?;
        let cartridge = Rc::new(RefCell::new(cartridge));
        let ppubus = PPUBus::new(cartridge.clone());

        let tv = TV::new(TV_WIDTH, TV_HEIGHT, |color| {
            [color.r, color.g, color.b, 0xFF]
        });
        let image = tv.get_image_clone();

        let ppu = PPU2C02::new(ppubus, tv);

        let ppu = Rc::new(RefCell::new(ppu));

        let apu = Rc::new(RefCell::new(APU2A03::new()));

        let ctrl = Controller::new();
        let ctrl_state = ctrl.get_primary_controller_state();

        let cpubus = CPUBus::new(cartridge.clone(), ppu.clone(), apu.clone(), ctrl);

        let cpu = CPU6502::new(
            Rc::new(RefCell::new(cpubus)),
            ppu.clone(),
            cartridge.clone(),
        );

        Ok(Self {
            cpu,
            ppu,
            apu,
            image,
            ctrl_state,
        })
    }

    /// calculate a new view based on the window size
    fn get_view(
        window_width: u32,
        window_height: u32,
        target_width: u32,
        target_height: u32,
    ) -> SfBox<View> {
        let mut viewport = FloatRect::new(0., 0., 1., 1.);

        let screen_width = window_width as f32 / target_width as f32;
        let screen_height = window_height as f32 / target_height as f32;

        if screen_width > screen_height {
            viewport.width = screen_height / screen_width;
            viewport.left = (1. - viewport.width) / 2.;
        } else if screen_height > screen_width {
            viewport.height = screen_width / screen_height;
            viewport.top = (1. - viewport.height) / 2.;
        }

        let mut view = View::new(
            Vector2f::new((TV_WIDTH / 2) as f32, (TV_HEIGHT / 2) as f32),
            Vector2f::new((TV_WIDTH) as f32, (TV_HEIGHT) as f32),
        );

        view.set_viewport(&viewport);

        view
    }

    pub fn run(&mut self) {
        self.cpu.reset();
        // Run the sound thread
        self.apu.borrow().play();

        let image = self.image.clone();
        let ctrl_state = self.ctrl_state.clone();

        let (tx, rx) = std::sync::mpsc::channel::<bool>();

        std::thread::spawn(move || {
            let mut window = RenderWindow::new(
                (
                    TV_WIDTH * SCREEN_SIZE_INCREASE,
                    TV_HEIGHT * SCREEN_SIZE_INCREASE,
                ),
                "NES test",
                Style::CLOSE | Style::RESIZE,
                &Default::default(),
            );
            window.set_vertical_sync_enabled(true);
            window.set_framerate_limit(60);

            // to scale the view into the window
            // this view is in the size of the NES TV
            // but we can scale the window and all the pixels will be scaled
            // accordingly
            window.set_view(&Self::get_view(
                window.size().x,
                window.size().y,
                TV_WIDTH,
                TV_HEIGHT,
            ));

            let mut texture = Texture::new(TV_WIDTH, TV_HEIGHT).expect("texture");

            'main: loop {
                if let Ok(mut ctrl) = ctrl_state.lock() {
                    while let Some(event) = window.poll_event() {
                        match event {
                            Event::Closed => break 'main,
                            Event::Resized { width, height } => {
                                window
                                    .set_view(&Self::get_view(width, height, TV_WIDTH, TV_HEIGHT));
                            }
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
                            Event::JoystickButtonPressed {
                                joystickid: 0,
                                button,
                            } => match button {
                                0 => ctrl.press(StandardNESKey::B),
                                1 => ctrl.press(StandardNESKey::A),
                                8 => ctrl.press(StandardNESKey::Select),
                                9 => ctrl.press(StandardNESKey::Start),
                                _ => {}
                            },
                            Event::JoystickButtonReleased {
                                joystickid: 0,
                                button,
                            } => match button {
                                0 => ctrl.release(StandardNESKey::B),
                                1 => ctrl.release(StandardNESKey::A),
                                8 => ctrl.release(StandardNESKey::Select),
                                9 => ctrl.release(StandardNESKey::Start),
                                _ => {}
                            },
                            Event::JoystickMoved {
                                joystickid: 0,
                                axis,
                                position,
                            } => match axis {
                                Axis::PovY => {
                                    if position > 0. {
                                        ctrl.press(StandardNESKey::Down)
                                    } else if position < 0. {
                                        ctrl.press(StandardNESKey::Up)
                                    } else {
                                        ctrl.release(StandardNESKey::Down);
                                        ctrl.release(StandardNESKey::Up);
                                    }
                                }
                                Axis::PovX => {
                                    if position > 0. {
                                        ctrl.press(StandardNESKey::Right)
                                    } else if position < 0. {
                                        ctrl.press(StandardNESKey::Left)
                                    } else {
                                        ctrl.release(StandardNESKey::Right);
                                        ctrl.release(StandardNESKey::Left);
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }

                window.clear(Color::BLACK);

                {
                    let pixels = &image.lock().unwrap();

                    let image =
                        Image::create_from_pixels(TV_WIDTH, TV_HEIGHT, pixels).expect("image");

                    texture.update_from_image(&image, 0, 0);
                }

                window.draw(&Sprite::with_texture(&texture));

                window.display();
            }

            tx.send(true).unwrap();
        });

        let mut last = std::time::Instant::now();
        const CPU_FREQ: f64 = 1.789773 * 1E6;
        const N: usize = 2000; // number of CPU cycles per loop, lower is smoother
        const CPU_PER_CYCLE_NANOS: f64 = 1E9 / CPU_FREQ;
        let mut apu_clock = false;

        // run the emulator loop
        while let Err(_) = rx.try_recv() {
            for _ in 0..N {
                self.cpu.run_next();
                if apu_clock {
                    self.apu.borrow_mut().clock();
                }
                apu_clock = !apu_clock;

                let mut ppu = self.ppu.borrow_mut();
                ppu.clock();
                ppu.clock();
                ppu.clock();
            }

            if let Some(d) =
                std::time::Duration::from_nanos((CPU_PER_CYCLE_NANOS * N as f64) as u64)
                    .checked_sub(last.elapsed())
            {
                std::thread::sleep(d);
            }

            last = std::time::Instant::now();
        }
    }
}
