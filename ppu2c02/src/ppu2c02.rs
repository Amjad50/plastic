use crate::ppu2c02_registers::Register;
use common::{Bus, Device};
use std::cell::Cell;

pub struct PPU2C02<'a, T: Bus> {
    // memory mapped registers
    reg_control: u8,
    reg_mask: u8,
    reg_status: Cell<u8>,
    reg_oma_addr: u8,
    reg_oma_data: u8,
    // reg_ppu_data: u8,
    reg_oma_dma: u8,

    scanline: u16,
    cycle: u16,

    // FIXME: get a better solution for vram address cur and tmp
    vram_address_cur: Cell<u16>,
    vram_address_tmp: u16,

    nametable_selector: u8, // 0, 1, 2, or 3 which maps to 0x2000, 0x2400, 0x2800, 0x2C00
    x_scroll: u8,
    y_scroll: u8,

    w_toggle: Cell<bool>, // this is used for registers that require 2 writes

    bg_pattern_shift_registers: [u16; 2],
    bg_palette_attribute_shift_registers: [u8; 2],

    bus: &'a mut T,
}

impl<'a, T> PPU2C02<'a, T>
where
    T: Bus,
{
    pub fn new(bus: &'a mut T) -> Self {
        Self {
            reg_control: 0,
            reg_mask: 0,
            reg_status: Cell::new(0),
            reg_oma_addr: 0,
            reg_oma_data: 0,
            // reg_ppu_data: 0,
            reg_oma_dma: 0,

            scanline: 261, // start from -1 scanline
            cycle: 0,

            vram_address_cur: Cell::new(0),
            vram_address_tmp: 0,

            nametable_selector: 0,
            x_scroll: 0,
            y_scroll: 0,

            w_toggle: Cell::new(false),

            bg_pattern_shift_registers: [0; 2],
            bg_palette_attribute_shift_registers: [0; 2],

            bus: bus,
        }
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            Register::Status => {
                // reset w_mode
                self.w_toggle.set(false);

                let result = self.reg_status.get();
                //  reading the status register will clear bit 7
                self.reg_status.set(self.reg_status.get() & 0x7F);
                result
            }
            Register::OmaData => self.reg_oma_data,
            Register::PPUData => {
                let result = self.read_bus(self.vram_address_cur.get());
                if self.reg_control & 0b100 != 0 {
                    // increment by 1
                    self.vram_address_cur.set(self.vram_address_cur.get() + 1);
                } else {
                    //increment by 32
                    self.vram_address_cur.set(self.vram_address_cur.get() + 32);
                }
                result
            }
            _ => {
                // unreadable
                0
            }
        }
    }

    pub(crate) fn write_register(&mut self, register: Register, data: u8) {
        match register {
            // After power/reset, writes to this register are ignored for about 30,000 cycles
            // TODO: not sure, if I should account for that
            Register::Control => {
                self.reg_control = data;
                self.nametable_selector = data & 0b11;

                // update temp address
                self.vram_address_tmp &= 0x3FF;
                self.vram_address_tmp |= (self.nametable_selector as u16) << 10;
            }
            Register::Mask => self.reg_mask = data,
            Register::OmaAddress => self.reg_oma_addr = data,
            Register::OmaData => self.reg_oma_data = data,
            Register::Scroll => {
                if self.w_toggle.get() {
                    self.x_scroll = data;

                    // update temp address
                    self.vram_address_tmp &= 0xFFFE; // lower 5 bits
                    self.vram_address_tmp |= (data >> 3) as u16;
                } else {
                    self.y_scroll = data;

                    // update temp address
                    self.vram_address_tmp &= 0xFC1F; // second 5 bits
                    self.vram_address_tmp |= ((data as u16) << 3) & 0b11111000;
                }

                self.w_toggle.set(!self.w_toggle.get());
            }
            Register::PPUAddress => {
                if self.w_toggle.get() {
                    // zero out the bottom 8 bits
                    self.vram_address_tmp &= 0xff00;
                    // set the data from the parameters
                    self.vram_address_tmp |= data as u16;
                } else {
                    // zero out the bottom 8 bits
                    self.vram_address_tmp &= 0x00ff;
                    // set the data from the parameters
                    self.vram_address_tmp |= (data as u16) << 8;

                    // copy to the current vram address
                    *self.vram_address_cur.get_mut() = self.vram_address_tmp;
                }

                self.w_toggle.set(!self.w_toggle.get());
            }
            Register::PPUData => {
                self.write_bus(self.vram_address_cur.get(), data);
                if self.reg_control & 0b100 != 0 {
                    // increment by 1
                    *self.vram_address_cur.get_mut() += 1;
                } else {
                    //increment by 32
                    *self.vram_address_cur.get_mut() += 32;
                }
            }
            Register::DmaOma => self.reg_oma_dma = data,
            _ => {
                // unwritable
            }
        };
    }

    fn read_bus(&self, address: u16) -> u8 {
        self.bus.read(address, Device::PPU)
    }

    fn write_bus(&mut self, address: u16, data: u8) {
        self.bus.write(address, data, Device::PPU);
    }

    /*
    ## PPU pattern table addressing ##
    DCBA98 76543210
    ---------------
    0HRRRR CCCCPTTT
    |||||| |||||+++- T: Fine Y offset, the row number within a tile
    |||||| ||||+---- P: Bit plane (0: "lower"; 1: "upper")
    |||||| ++++----- C: Tile column
    ||++++---------- R: Tile row
    |+-------------- H: Half of sprite table (0: "left"; 1: "right")
    +--------------- 0: Pattern table is at $0000-$1FFF
    */
    fn fetch_pattern_background(&self, location: u8) -> [u8; 2] {
        let fine_y = (self.y_scroll & 0b111) as u16;

        // for background
        let pattern_table_selector = if self.reg_control & 0x10 != 0 { 1 } else { 0 };

        let low_plane_pattern =
            self.read_bus(pattern_table_selector << 12 | (location as u16) << 4 | 0 << 3 | fine_y);

        let high_plane_pattern =
            self.read_bus(pattern_table_selector << 12 | (location as u16) << 4 | 1 << 3 | fine_y);

        [low_plane_pattern, high_plane_pattern]
    }

    /*
    ## Attribute address ##
    NN 1111 YYY XXX
    || |||| ||| +++-- high 3 bits of coarse X (x/4)
    || |||| +++------ high 3 bits of coarse Y (y/4)
    || ++++---------- attribute offset (960 bytes)
    ++--------------- nametable select
    */
    fn fetch_attribute_byte(&self) -> u8 {
        let x = (self.x_scroll >> 5) as u16;
        let y = (self.y_scroll >> 5) as u16;

        self.read_bus((self.nametable_selector as u16) << 10 | 0xF << 6 | y << 3 | x)
    }
    /*
    ## color location offset 0x3F00 ##
    43210
    |||||
    |||++- Pixel value from tile data
    |++--- Palette number from attribute table or OAM
    +----- Background/Sprite select
    */
    fn get_pixel(&mut self) -> u8 {
        let fine_x = self.x_scroll & 0b111;
        let low_plane_bit = ((self.bg_pattern_shift_registers[0] >> fine_x as u16) & 0x1) as u8;
        let high_plane_bit = ((self.bg_pattern_shift_registers[1] >> fine_x as u16) & 0x1) as u8;

        let color_bit = high_plane_bit << 1 | low_plane_bit;

        let current_attribute = self.bg_palette_attribute_shift_registers[0];
        let attribute_location_x = (self.x_scroll >> 1) & 0xf;
        let attribute_location_y = (self.y_scroll >> 1) & 0xf;

        let attribute_location = attribute_location_y << 1 | attribute_location_x;

        let palette = (current_attribute >> attribute_location) & 0b11;
        let background = 1; // true

        let color =
            self.read_bus(0x3F00 | (background << 4 | palette << 2 | color_bit << 2) as u16);

        // advance the shift registers
        for i in 0..=1 {
            self.bg_pattern_shift_registers[i] >>= 1;
        }

        color
    }

    fn render_pixel(&mut self) {
        let color = self.get_pixel();
        // render this color
    }

    // run one cycle, this should be fed from Master clock
    pub fn run_cycle(&mut self) {
        // current scanline
        match self.scanline {
            261 => {
                // pre-render

                // reset y_scroll from tmp
                self.y_scroll &= 0b111; // keep fine y only
                self.y_scroll |= ((self.vram_address_tmp >> 5) & 0b11111) as u8;

                // update temp address
                *self.vram_address_cur.get_mut() &= 0xFC1F; // second 5 bits
                *self.vram_address_cur.get_mut() |= ((self.y_scroll as u16) << 3) & 0b11111000;
                // next round
                self.scanline = 0;
            }
            0..=239 => {
                // render
                self.run_render_cycle();
            }
            240 => {
                // post-render
                // idle
            }
            241..=260 => {
                // vertical blanking
                if self.cycle == 1 && self.scanline == 241 {
                    // set v-blank
                    *self.reg_status.get_mut() |= 0x80;

                    // FIXME: raise non-maskable interrupt to the CPU
                }
            }
            _ => {
                unreachable!();
            }
        }
        self.cycle += 1;
        if self.cycle > 340 {
            self.scanline += 1;
            self.cycle = 0;
        }
    }

    // run one cycle which is part of a scanline execution
    fn run_render_cycle(&mut self) {
        match self.cycle {
            0 => {
                // idle
            }
            1..=256 => {
                // main render
                self.render_pixel();

                // fetch and reload shift registers
                if self.cycle % 8 == 0 {
                    let nametable_tile = self.read_bus(self.vram_address_cur.get());
                    let tile_pattern = self.fetch_pattern_background(nametable_tile);
                    let attribute_byte = self.fetch_attribute_byte();

                    // update th shift registers
                    for i in 0..=1 {
                        self.bg_pattern_shift_registers[i] &= 0xff;
                        self.bg_pattern_shift_registers[i] |= (tile_pattern[i] as u16) << 8;
                    }

                    // reload attribute shift register
                    // TODO: this does not seem like a shift register but not sure
                    self.bg_palette_attribute_shift_registers[0] =
                        self.bg_palette_attribute_shift_registers[1];
                    self.bg_palette_attribute_shift_registers[1] = attribute_byte;

                    // increment scrolling
                    if self.cycle != 256 {
                        self.x_scroll += 1;
                        // increase X in vram current address
                        *self.vram_address_cur.get_mut() += 1;
                    } else {
                        self.y_scroll += 1;

                        // update vram current address
                        *self.vram_address_cur.get_mut() &= 0xFC1F; // second 5 bits
                        *self.vram_address_cur.get_mut() |=
                            ((self.y_scroll as u16) << 3) & 0b11111000;
                    }
                }
            }
            257..=320 => {
                if self.cycle == 257 {
                    let fine_x = self.x_scroll & 0x7; // save
                    self.x_scroll = ((self.vram_address_tmp & 0b11111) << 3) as u8;
                    self.x_scroll |= fine_x; //restore
                }
            }
            321..=340 => {
                // lets just do it in the beginning
                if self.cycle == 321 {
                    // load next 2 bytes
                    for _ in 0..2 {
                        let nametable_tile = self.read_bus(self.vram_address_cur.get());
                        let tile_pattern = self.fetch_pattern_background(nametable_tile);
                        let attribute_byte = self.fetch_attribute_byte();

                        // update th shift registers
                        for i in 0..=1 {
                            self.bg_pattern_shift_registers[i] >>= 8;
                            self.bg_pattern_shift_registers[i] |= (tile_pattern[i] as u16) << 8;
                        }

                        // reload attribute shift register
                        // TODO: this does not seem like a shift register but not sure
                        self.bg_palette_attribute_shift_registers[0] =
                            self.bg_palette_attribute_shift_registers[1];
                        self.bg_palette_attribute_shift_registers[1] = attribute_byte;

                        // increment scrolling
                        self.x_scroll += 1;
                        // increase X in vram current address
                        *self.vram_address_cur.get_mut() += 1;
                    }
                }
            }
            _ => {
                unreachable!();
            }
        }
    }

    /*
    ## PPU VRAM top 12-bit address ## (v and t)
    NN YYYYY XXXXX
    || ||||| +++++-- coarse X scroll
    || +++++-------- coarse Y scroll
    ++-------------- nametable select


    tile address      = 0x2000 | (v & 0x0FFF)
    attribute address = 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07)
    */
}
