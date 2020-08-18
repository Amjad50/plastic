use apu2a03::APU2A03;
use cartridge::{Cartridge, CartridgeError};
use common::{Bus, Device, MirroringProvider};
use controller::{Controller, StandardNESControllerState};
use cpu6502::CPU6502;
use display::TV;
use ppu2c02::{Palette, VRam, PPU2C02};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc::channel, Arc, Mutex};

use crate::{UiEvent, UiProvider};

// NES TV size
// TODO: should be included in "tv" crate
pub const TV_WIDTH: u32 = 256;
pub const TV_HEIGHT: u32 = 240;

struct PPUBus {
    cartridge: Rc<RefCell<dyn Bus>>,
    vram: VRam,
    palettes: Palette,
}

struct CPUBus {
    cartridge: Rc<RefCell<dyn Bus>>,
    ram: [u8; 0x800],
    ppu: Rc<RefCell<dyn Bus>>,
    apu: Rc<RefCell<dyn Bus>>,
    contoller: Controller,
}

impl CPUBus {
    pub fn new(
        cartridge: Rc<RefCell<dyn Bus>>,
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
    pub fn new<S>(cartridge: Rc<RefCell<S>>) -> Self
    where
        S: Bus + MirroringProvider + 'static,
    {
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

pub struct NES<P: UiProvider + Send + 'static> {
    cartridge: Rc<RefCell<Cartridge>>,
    cpu: CPU6502<CPUBus>,
    ppu: Rc<RefCell<PPU2C02<PPUBus>>>,
    apu: Rc<RefCell<APU2A03>>,
    image: Arc<Mutex<Vec<u8>>>,
    ctrl_state: Arc<Mutex<StandardNESControllerState>>,

    ui: Option<P>, // just to hold the UI object (it will be taken in the main loop)

    paused: bool,
}

impl<P: UiProvider + Send + 'static> NES<P> {
    pub fn new(filename: &str, ui: P) -> Result<Self, CartridgeError> {
        let cartridge = Cartridge::from_file(filename)?;

        Ok(Self::create_nes(cartridge, ui))
    }

    pub fn new_without_file(ui: P) -> Self {
        let cartridge = Cartridge::new_without_file();

        Self::create_nes(cartridge, ui)
    }

    fn create_nes(cartridge: Cartridge, ui: P) -> Self {
        let cartridge = Rc::new(RefCell::new(cartridge));
        let ppubus = PPUBus::new(cartridge.clone());

        let tv = TV::new(TV_WIDTH, TV_HEIGHT, P::get_tv_color_converter());
        let image = tv.get_image_clone();

        let ppu = PPU2C02::new(ppubus, tv);

        let ppu = Rc::new(RefCell::new(ppu));

        let apu = Rc::new(RefCell::new(APU2A03::new()));

        let ctrl = Controller::new();
        let ctrl_state = ctrl.get_primary_controller_state();

        let cpubus = CPUBus::new(cartridge.clone(), ppu.clone(), apu.clone(), ctrl);

        let mut cpu = CPU6502::new(Rc::new(RefCell::new(cpubus)), ppu.clone(), apu.clone());
        cpu.add_irq_provider(cartridge.clone());
        cpu.add_irq_provider(apu.clone());

        let paused = cartridge.borrow().is_empty();

        Self {
            cartridge,
            cpu,
            ppu,
            apu,
            image,
            ctrl_state,
            ui: Some(ui),

            paused,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset();

        let ppubus = PPUBus::new(self.cartridge.clone());

        self.ppu.borrow_mut().reset(ppubus);

        self.apu.replace(APU2A03::new());

        self.paused = self.cartridge.borrow().is_empty();
        // TODO: implement reset for cartridge if needed
        // self.cartridge.borrow_mut().reset();
        // FIXME: added a way to reset CPU BUS
    }

    /// calculate a new view based on the window size
    pub fn run(&mut self) {
        let image = self.image.clone();
        let ctrl_state = self.ctrl_state.clone();

        let (ui_to_nes_sender, ui_to_nes_receiver) = channel::<UiEvent>();

        let mut ui = self.ui.take().unwrap();

        std::thread::spawn(move || {
            ui.run_ui_loop(ui_to_nes_sender.clone(), image, ctrl_state);
            ui_to_nes_sender.send(UiEvent::Exit).unwrap();
        });

        self.cpu.reset();
        self.apu.borrow_mut().update_apu_freq(1.7 * 1E6 / 2.);

        if !self.paused {
            // Run the sound thread
            self.apu.borrow().play();
        }

        let mut last = std::time::Instant::now();
        const CPU_FREQ: f64 = 1.789773 * 1E6;
        const N: usize = 2000; // number of CPU cycles per loop, lower is smoother
        const CPU_PER_CYCLE_NANOS: f64 = 1E9 / CPU_FREQ;

        let mut average_apu_freq = CPU_FREQ / 2.;
        let mut average_counter = 1.;

        // run the emulator loop
        loop {
            // check for events
            if let Ok(event) = ui_to_nes_receiver.try_recv() {
                match event {
                    UiEvent::Exit => break,
                    UiEvent::Reset => {
                        self.reset();
                        if !self.paused {
                            self.apu.borrow().play();
                        }
                        average_apu_freq = CPU_FREQ / 2.;
                        average_counter = 1.;
                    }

                    UiEvent::LoadRom(file_location) => {
                        let cartridge = Cartridge::from_file(file_location);
                        if let Ok(cartridge) = cartridge {
                            self.cartridge.replace(cartridge);
                            self.reset();
                            if !self.paused {
                                self.apu.borrow().play();
                            }
                            average_apu_freq = CPU_FREQ / 2.;
                            average_counter = 1.;
                        } else {
                            break;
                        }
                    }
                }
            }

            if self.paused {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }

            for _ in 0..N {
                self.cpu.run_next();
                self.cpu.run_next();

                self.apu.borrow_mut().clock();

                {
                    let mut ppu = self.ppu.borrow_mut();
                    ppu.clock();
                    ppu.clock();
                    ppu.clock();
                    ppu.clock();
                    ppu.clock();
                    ppu.clock();
                }
            }

            if let Some(d) =
                std::time::Duration::from_nanos((CPU_PER_CYCLE_NANOS * 2. * N as f64) as u64)
                    .checked_sub(last.elapsed())
            {
                std::thread::sleep(d);
            }

            let apu_freq = N as f64 / last.elapsed().as_secs_f64();

            average_counter += 1.;
            average_apu_freq = average_apu_freq + ((apu_freq - average_apu_freq) / average_counter);

            self.apu.borrow_mut().update_apu_freq(average_apu_freq);

            last = std::time::Instant::now();
        }
    }
}
