use crate::ppu2c02_registers::Register;
use common::{interconnection::PPUCPUConnection, Bus, Device};
use display::{COLORS, TV};
use std::cell::Cell;

bitflags! {
    pub struct ControlReg: u8 {
        const BASE_NAMETABLE = 0b00000011;
        const VRAM_INCREMENT = 0b00000100;
        const SPRITE_PATTERN_ADDRESS = 0b00001000;
        const BACKGROUND_PATTERN_ADDRESS = 0b00010000;
        const SPRITE_SIZE = 0b00100000;
        const MASTER_SLAVE_SELECT = 0b01000000;
        const GENERATE_NMI_ENABLE = 0b10000000;
    }
}

impl ControlReg {
    pub fn base_nametable_address(&self) -> u16 {
        // 0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00
        0x2000 | ((self.bits & Self::BASE_NAMETABLE.bits) as u16) << 10
    }

    pub fn vram_increment(&self) -> u16 {
        if self.intersects(Self::VRAM_INCREMENT) {
            32
        } else {
            1
        }
    }

    pub fn sprite_pattern_address(&self) -> u16 {
        ((self.bits & Self::SPRITE_PATTERN_ADDRESS.bits) as u16) << 12
    }

    pub fn background_pattern_address(&self) -> u16 {
        ((self.bits & Self::BACKGROUND_PATTERN_ADDRESS.bits) as u16) << 12
    }

    pub fn nmi_enabled(&self) -> bool {
        self.intersects(Self::GENERATE_NMI_ENABLE)
    }
}

bitflags! {
    pub struct MaskReg: u8 {
        const GRAYSCALE_ENABLE = 0b00000001;
        const SHOW_BACKGROUND_LEFTMOST_8 = 0b00000010;
        const SHOW_SPRITES_LEFTMOST_8 = 0b00000100;
        const SHOW_BACKGROUND = 0b00001000;
        const SHOW_SPRITES = 0b00010000;
        const EMPHASIZE_RED = 0b00100000;
        const EMPHASIZE_GREEN = 0b01000000;
        const EMPHASIZE_BLUE = 0b10000000;
    }
}

impl MaskReg {
    pub fn background_enabled(&self) -> bool {
        self.intersects(Self::SHOW_BACKGROUND)
    }

    pub fn sprites_enabled(&self) -> bool {
        self.intersects(Self::SHOW_BACKGROUND)
    }

    pub fn rendering_enabled(&self) -> bool {
        self.background_enabled() || self.sprites_enabled()
    }
}

bitflags! {
    pub struct StatusReg: u8 {
        const SPRITE_OVERFLOW = 0b00100000;
        const SPRITE_0_HIT = 0b01000000;
        const VERTICAL_BLANK = 0b10000000;
    }
}

pub struct PPU2C02<T: Bus> {
    // memory mapped registers
    reg_control: ControlReg,
    reg_mask: MaskReg,
    reg_status: Cell<StatusReg>,
    reg_oma_addr: u8,
    reg_oma_data: u8,
    reg_oma_dma: u8,

    scanline: u16,
    cycle: u16,

    // FIXME: get a better solution for vram address cur and tmp
    vram_address_cur: Cell<u16>,
    vram_address_tmp: u16,

    x_scroll: u8,
    y_scroll: u8,

    w_toggle: Cell<bool>, // this is used for registers that require 2 writes

    bg_pattern_shift_registers: [u16; 2],
    bg_palette_attribute_shift_registers: [u8; 2],

    nmi_pin_status: bool,

    bus: T,
    tv: TV,
}

impl<T> PPU2C02<T>
where
    T: Bus,
{
    pub fn new(bus: T, tv: TV) -> Self {
        Self {
            reg_control: ControlReg::empty(),
            reg_mask: MaskReg::empty(),
            reg_status: Cell::new(StatusReg::empty()),
            reg_oma_addr: 0,
            reg_oma_data: 0,
            reg_oma_dma: 0,

            scanline: 261, // start from -1 scanline
            cycle: 0,

            vram_address_cur: Cell::new(0),
            vram_address_tmp: 0,

            x_scroll: 0,
            y_scroll: 0,

            w_toggle: Cell::new(false),

            bg_pattern_shift_registers: [0; 2],
            bg_palette_attribute_shift_registers: [0; 2],

            nmi_pin_status: false,

            bus,
            tv,
        }
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            Register::Status => {
                // reset w_mode
                self.w_toggle.set(false);

                let result = self.reg_status.get().bits;
                //  reading the status register will clear bit 7
                self.reg_status
                    .set(StatusReg::from_bits(result & 0x7F).unwrap());

                result
            }
            Register::OmaData => self.reg_oma_data,
            Register::PPUData => {
                let result = self.read_bus(self.vram_address_cur.get());

                self.increment_vram_readwrite();

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
                self.reg_control.bits = data;
            }
            Register::Mask => self.reg_mask.bits = data,
            Register::OmaAddress => self.reg_oma_addr = data,
            Register::OmaData => self.reg_oma_data = data,
            Register::Scroll => {
                if self.w_toggle.get() {
                    // w == 1

                    self.y_scroll = data;
                } else {
                    // w == 0

                    self.x_scroll = data;
                }

                self.w_toggle.set(!self.w_toggle.get());
            }
            Register::PPUAddress => {
                if self.w_toggle.get() {
                    // w == 1

                    // zero out the bottom 8 bits
                    self.vram_address_tmp &= 0xff00;
                    // set the data from the parameters
                    self.vram_address_tmp |= data as u16;

                    // copy to the current vram address
                    *self.vram_address_cur.get_mut() = self.vram_address_tmp;
                } else {
                    // w == 0

                    // zero out the top 8 bits
                    self.vram_address_tmp &= 0x00ff;
                    // set the data from the parameters
                    self.vram_address_tmp |= (data as u16) << 8;
                }

                self.w_toggle.set(!self.w_toggle.get());
            }
            Register::PPUData => {
                self.write_bus(self.vram_address_cur.get(), data);
                self.increment_vram_readwrite();
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

    fn increment_vram_readwrite(&self) {
        // only increment if its valid, and increment by the correct ammount
        if self.scanline > 240 || !self.reg_mask.rendering_enabled() {
            self.vram_address_cur
                .set(self.vram_address_cur.get() + self.reg_control.vram_increment());
        }
    }

    fn increment_vram_coarse_scroll_x(&mut self) {
        // extract coarse_x
        let mut coarse_x = self.vram_address_cur.get() & 0b11111; // only first 5 bits
        coarse_x += 1;

        // clear first 5 bits
        *self.vram_address_cur.get_mut() &= 0xFFE0;
        // put result back
        *self.vram_address_cur.get_mut() |= (coarse_x & 0b11111) as u16;
    }

    // return carry
    fn increment_vram_coarse_scroll_y(&mut self) {
        // extract coarse_y
        let mut coarse_y = (self.vram_address_cur.get() & 0b1111100000) >> 5; // only second 5 bits
        coarse_y += 1;

        // clear second 5 bits
        *self.vram_address_cur.get_mut() &= 0xFC1F;
        // put result back
        *self.vram_address_cur.get_mut() |= ((coarse_y & 0b11111) as u16) << 5;
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
        let pattern_table = self.reg_control.background_pattern_address();

        let low_plane_pattern =
            self.read_bus(pattern_table | (location as u16) << 4 | 0 << 3 | fine_y);

        let high_plane_pattern =
            self.read_bus(pattern_table | (location as u16) << 4 | 1 << 3 | fine_y);

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

        self.read_bus(self.reg_control.base_nametable_address() | 0xF << 6 | y << 3 | x)
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
        let low_plane_bit =
            ((self.bg_pattern_shift_registers[0] >> (15 - fine_x) as u16) & 0x1) as u8;
        let high_plane_bit =
            ((self.bg_pattern_shift_registers[1] >> (15 - fine_x) as u16) & 0x1) as u8;

        let color_bit = high_plane_bit << 1 | low_plane_bit;

        let current_attribute = self.bg_palette_attribute_shift_registers[0];
        let attribute_location_x = (self.x_scroll >> 1) & 0x1;
        let attribute_location_y = (self.y_scroll >> 1) & 0x1;

        let attribute_location = attribute_location_y << 1 | attribute_location_x;

        let palette = (current_attribute >> attribute_location) & 0b11;
        let background = 0;

        let color = self.read_bus(0x3F00 | (background << 4 | palette << 2 | color_bit) as u16);

        // advance the shift registers
        for i in 0..=1 {
            self.bg_pattern_shift_registers[i] = self.bg_pattern_shift_registers[i].wrapping_shl(1);
        }

        color
    }

    fn render_pixel(&mut self) {
        let color = self.get_pixel();
        // render the color
        self.tv.set_pixel(
            self.cycle as u32,
            self.scanline as u32,
            &COLORS[color as usize],
        );
    }

    // run one cycle, this should be fed from Master clock
    pub fn run_cycle(&mut self) {
        // current scanline
        match self.scanline {
            261 => {
                // pre-render

                if self.cycle == 1 {
                    // use top-left y scrolling in current
                    *self.vram_address_cur.get_mut() &= 0xFC1F; // second 5 bits
                    *self.vram_address_cur.get_mut() |= ((self.y_scroll & 0b11111000) as u16) << 2;

                    // clear v-blank
                    self.reg_status.get_mut().remove(StatusReg::VERTICAL_BLANK);
                }
            }
            0..=239 => {
                // render only if allowed
                if self.reg_mask.rendering_enabled() {
                    self.run_render_cycle();
                }
            }
            240 => {
                // post-render
                // idle
            }
            241..=260 => {
                // vertical blanking
                if self.cycle == 1 && self.scanline == 241 {
                    // set v-blank
                    self.reg_status.get_mut().insert(StatusReg::VERTICAL_BLANK);

                    // if raising NMI is enabled
                    if self.reg_control.nmi_enabled() {
                        self.nmi_pin_status = true;
                    }
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

            // next frame
            if self.scanline > 261 {
                self.scanline = 0;
            }
        }
    }

    // run one cycle which is part of a scanline execution
    fn run_render_cycle(&mut self) {
        match self.cycle {
            0 => {
                // idle
            }
            1..=256 => {
                // fetch and reload shift registers
                if self.cycle % 8 == 0 {
                    let nametable_tile = self.read_bus(
                        self.reg_control.base_nametable_address()
                            | self.vram_address_cur.get() & 0x3FF,
                    );
                    let tile_pattern = self.fetch_pattern_background(nametable_tile);
                    let attribute_byte = self.fetch_attribute_byte();

                    // update th shift registers
                    for i in 0..=1 {
                        self.bg_pattern_shift_registers[i] &= 0xFF00;

                        // in this stage, because we reload in dots (8, 16, 24...)
                        // the shift registers will be shifted one more time
                        // meaning, it will be shifted 8 times
                        self.bg_pattern_shift_registers[i] |= tile_pattern[i] as u16;
                    }

                    // reload attribute shift register
                    // TODO: this does not seem like a shift register but not sure
                    self.bg_palette_attribute_shift_registers[0] =
                        self.bg_palette_attribute_shift_registers[1];
                    self.bg_palette_attribute_shift_registers[1] = attribute_byte;

                    // increment scrolling X in current VRAM address
                    self.increment_vram_coarse_scroll_x();

                    // increment fine scrolling Y on the last dot
                    if self.cycle == 256 {
                        // increment fine without carry
                        let mut fine_y = self.y_scroll & 0b111;
                        fine_y += 1;

                        self.y_scroll &= 0b11111000;
                        self.y_scroll |= fine_y & 0b111;

                        // if the increment resulted in a carry, go to the next tile
                        // i.e. increment coarse Y
                        if fine_y & 0x8 != 0 {
                            self.increment_vram_coarse_scroll_y();
                        }
                    }
                }
            }
            257..=320 => {
                // unused
                if self.cycle == 257 {
                    // restore coarse X scrolling into current VRAM address
                    // to prepare for the next scanline
                    let coarse_x = self.x_scroll >> 3; // only top 5 bits

                    *self.vram_address_cur.get_mut() &= 0xFFE0; // first 5 bits
                    *self.vram_address_cur.get_mut() |= (coarse_x & 0b11111) as u16;
                }
            }
            321..=340 => {
                // lets just do it in the beginning
                if self.cycle == 321 {
                    // load next 2 bytes
                    for _ in 0..2 {
                        let nametable_tile = self.read_bus(
                            self.reg_control.base_nametable_address()
                                | self.vram_address_cur.get() & 0x3FF,
                        );
                        let tile_pattern = self.fetch_pattern_background(nametable_tile);
                        let attribute_byte = self.fetch_attribute_byte();

                        // update th shift registers
                        for i in 0..=1 {
                            self.bg_pattern_shift_registers[i] =
                                self.bg_pattern_shift_registers[i].wrapping_shl(8);
                            self.bg_pattern_shift_registers[i] |= tile_pattern[i] as u16;
                        }

                        // reload attribute shift register
                        // TODO: this does not seem like a shift register but not sure
                        self.bg_palette_attribute_shift_registers[0] =
                            self.bg_palette_attribute_shift_registers[1];
                        self.bg_palette_attribute_shift_registers[1] = attribute_byte;

                        self.increment_vram_coarse_scroll_x();
                    }
                }
            }
            _ => {
                unreachable!();
            }
        }

        // render after reloading
        if self.cycle <= 255 {
            // main render
            self.render_pixel();
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

impl<T> PPUCPUConnection for PPU2C02<T>
where
    T: Bus,
{
    fn is_nmi_pin_set(&self) -> bool {
        self.nmi_pin_status
    }
    fn clear_nmi_pin(&mut self) {
        self.nmi_pin_status = false;
    }
}
