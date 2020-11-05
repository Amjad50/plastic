use crate::ppu2c02_registers::Register;
use crate::sprite::{Sprite, SpriteAttribute};
use common::{
    interconnection::PPUCPUConnection,
    save_state::{Savable, SaveError},
    Bus, Device,
};
use display::{Color, COLORS, TV};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::cmp::min;

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
    pub fn nametable_selector(&self) -> u8 {
        // 0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00
        self.bits & Self::BASE_NAMETABLE.bits
    }

    pub fn vram_increment(&self) -> u16 {
        if self.intersects(Self::VRAM_INCREMENT) {
            32
        } else {
            1
        }
    }

    pub fn sprite_pattern_address(&self) -> u16 {
        ((self.intersects(Self::SPRITE_PATTERN_ADDRESS)) as u16) << 12
    }

    pub fn background_pattern_address(&self) -> u16 {
        ((self.intersects(Self::BACKGROUND_PATTERN_ADDRESS)) as u16) << 12
    }

    pub fn nmi_enabled(&self) -> bool {
        self.intersects(Self::GENERATE_NMI_ENABLE)
    }

    pub fn sprite_height(&self) -> u8 {
        // if SPRITE_SIZE is 1, then it will be (8 << 1) == 16, else it will be 8
        8 << self.intersects(Self::SPRITE_SIZE) as u8
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
        self.intersects(Self::SHOW_SPRITES)
    }

    pub fn rendering_enabled(&self) -> bool {
        self.background_enabled() || self.sprites_enabled()
    }

    /// returns true if the background pixels on the left should not
    /// be shown, false otherwise
    pub fn background_left_clipping_enabled(&self) -> bool {
        !self.intersects(Self::SHOW_BACKGROUND_LEFTMOST_8)
    }

    /// returns true if the sprite pixels on the left should not
    /// be shown, false otherwise
    pub fn sprites_left_clipping_enabled(&self) -> bool {
        !self.intersects(Self::SHOW_SPRITES_LEFTMOST_8)
    }

    pub fn is_grayscale(&self) -> bool {
        self.intersects(Self::GRAYSCALE_ENABLE)
    }
}

bitflags! {
    pub struct StatusReg: u8 {
        const SPRITE_OVERFLOW = 0b00100000;
        const SPRITE_0_HIT = 0b01000000;
        const VERTICAL_BLANK = 0b10000000;
    }
}

pub struct PPU2C02<T: Bus + Savable> {
    // memory mapped registers
    reg_control: ControlReg,
    reg_mask: MaskReg,
    reg_status: Cell<StatusReg>,
    reg_oam_addr: Cell<u8>,

    scanline: u16,
    cycle: u16,

    /// ## PPU VRAM top 12-bit address ## (v and t)
    /// NN YYYYY XXXXX
    /// || ||||| +++++-- coarse X scroll
    /// || +++++-------- coarse Y scroll
    /// ++-------------- nametable select
    vram_address_cur: Cell<u16>,
    vram_address_top_left: u16,

    ppu_data_read_buffer: Cell<u8>,

    fine_x_scroll: u8,

    w_toggle: Cell<bool>, // this is used for registers that require 2 writes

    bg_pattern_shift_registers: [u16; 2],
    bg_palette_shift_registers: [u16; 2],

    nmi_pin_status: Cell<bool>,
    nmi_occured_in_this_frame: Cell<bool>,

    bus: T,
    tv: TV,

    primary_oam: [Sprite; 64],
    secondary_oam: [Sprite; 8],

    secondary_oam_counter: u8,

    sprite_pattern_shift_registers: [[u8; 2]; 8],
    sprite_attribute_registers: [SpriteAttribute; 8],
    sprite_counters: [u8; 8],
    sprite_0_present: bool,
    next_scanline_sprite_0_present: bool,

    is_dma_request: bool,
    dma_request_address: u8,

    is_odd_frame: bool,
}

impl<T> PPU2C02<T>
where
    T: Bus + Savable,
{
    pub fn new(bus: T, tv: TV) -> Self {
        Self {
            reg_control: ControlReg::empty(),
            reg_mask: MaskReg::empty(),
            reg_status: Cell::new(StatusReg::empty()),
            reg_oam_addr: Cell::new(0),

            // this would result in it starting from 0,0 next cycle
            scanline: 261, // start from -1 scanline
            cycle: 340,    // last cycle

            vram_address_cur: Cell::new(0),
            vram_address_top_left: 0,

            ppu_data_read_buffer: Cell::new(0),

            fine_x_scroll: 0,

            w_toggle: Cell::new(false),

            bg_pattern_shift_registers: [0; 2],
            bg_palette_shift_registers: [0; 2],

            nmi_pin_status: Cell::new(false),
            nmi_occured_in_this_frame: Cell::new(false),

            bus,
            tv,

            primary_oam: [Sprite::empty(); 64],
            secondary_oam: [Sprite::empty(); 8],

            secondary_oam_counter: 0,

            sprite_pattern_shift_registers: [[0; 2]; 8],
            sprite_attribute_registers: [SpriteAttribute::empty(); 8],
            sprite_counters: [0; 8],
            sprite_0_present: false,
            next_scanline_sprite_0_present: false,

            is_dma_request: false,
            dma_request_address: 0,

            is_odd_frame: false,
        }
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            Register::Status => {
                // reset w_mode
                self.w_toggle.set(false);

                if self.scanline == 241 {
                    // Race Condition Warning: Reading PPUSTATUS within two
                    // cycles of the start of vertical blank will return 0 in bit 7
                    // but clear the latch anyway, causing NMI to not occur that frame
                    if self.cycle <= 2 {
                        self.reg_status
                            .set(StatusReg::from_bits(self.reg_status.get().bits & 0x7F).unwrap());
                    }
                    // for NMI it has quite a different range
                    // source: tests
                    if self.cycle >= 2 && self.cycle <= 4 {
                        self.nmi_pin_status.set(false);
                        self.nmi_occured_in_this_frame.set(true);
                    }
                }
                let result = self.reg_status.get().bits;
                //  reading the status register will clear bit 7
                self.reg_status
                    .set(StatusReg::from_bits(result & 0x7F).unwrap());

                result
            }
            Register::OmaData => self.read_sprite_byte(self.reg_oam_addr.get()),
            Register::PPUData => {
                let address = self.vram_address_cur.get();
                let data_in_addr = self.read_bus(address);

                // only 0 - 0x2FFF (before palette) is buffered
                let result = if address <= 0x3EFF {
                    let tmp_result = self.ppu_data_read_buffer.get();

                    // fill buffer
                    self.ppu_data_read_buffer.set(data_in_addr);

                    tmp_result
                } else {
                    // reload buffer with VRAM address hidden by palette
                    // wrap to 0x2FFF rather than 0x3EFF, to avoid the mirror
                    self.ppu_data_read_buffer
                        .set(self.read_bus(address & 0x2FFF));
                    data_in_addr
                };

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

                // write nametable also in top_left vram address
                self.vram_address_top_left &= 0xF3FF;
                self.vram_address_top_left |= (self.reg_control.nametable_selector() as u16) << 10;

                // if the NMI flag is set, run immediate NMI to the CPU
                // but only run if we are in the VBLANK period and no
                // other NMI has occurred so far
                if self.reg_control.nmi_enabled() {
                    if self.reg_status.get().intersects(StatusReg::VERTICAL_BLANK)
                        && !self.nmi_occured_in_this_frame.get()
                    {
                        self.nmi_pin_status.set(true);
                        self.nmi_occured_in_this_frame.set(true);
                    }
                } else {
                    // if the NMI is disabled, stop the NMI (if the flag was set)
                    if self.scanline == 241 && self.cycle <= 4 {
                        self.nmi_pin_status.set(false);
                        self.nmi_occured_in_this_frame.set(true);
                    } else {
                        // in case if the NMI flag was disabled, then mark as nmi
                        // never occurred on this frame, even if it has
                        // meaning, that in some cases 2 NMI can occur
                        self.nmi_occured_in_this_frame.set(false);
                    }
                }
            }
            Register::Mask => self.reg_mask.bits = data,
            Register::OmaAddress => self.reg_oam_addr.set(data),
            Register::OmaData => {
                self.write_sprite_byte(self.reg_oam_addr.get(), data);
                if self.scanline > 240 || !self.reg_mask.rendering_enabled() {
                    *self.reg_oam_addr.get_mut() = self.reg_oam_addr.get().wrapping_add(1);
                }
            }
            Register::Scroll => {
                if self.w_toggle.get() {
                    // w == 1
                    self.set_top_left_y_scroll(data);
                } else {
                    // w == 0
                    self.set_top_left_x_scroll(data);
                }

                self.w_toggle.set(!self.w_toggle.get());
            }
            Register::PPUAddress => {
                if self.w_toggle.get() {
                    // w == 1

                    // zero out the bottom 8 bits
                    self.vram_address_top_left &= 0xff00;
                    // set the data from the parameters
                    self.vram_address_top_left |= data as u16;

                    // a dummy read to the cartridge as some mappers rely
                    // on PPU address pins for operations
                    let _ = self.read_bus(self.vram_address_top_left);

                    // copy to the current vram address
                    *self.vram_address_cur.get_mut() = self.vram_address_top_left;
                } else {
                    // w == 0

                    // zero out the top 8 bits
                    self.vram_address_top_left &= 0x00ff;
                    // set the data from the parameters
                    self.vram_address_top_left |= (data as u16) << 8;

                    // update nametable
                    self.reg_control.bits &= !(ControlReg::BASE_NAMETABLE.bits);
                    self.reg_control.bits |= (data >> 2) & 0b11;
                }

                self.w_toggle.set(!self.w_toggle.get());
            }
            Register::PPUData => {
                self.write_bus(self.vram_address_cur.get(), data);
                self.increment_vram_readwrite();
            }
            Register::DmaOma => {
                self.dma_request_address = data;
                self.is_dma_request = true;
            }
            _ => {
                // unwritable
            }
        };
    }

    /// expose the bus for reading only
    pub fn ppu_bus(&self) -> &T {
        &self.bus
    }

    fn read_bus(&self, address: u16) -> u8 {
        self.bus.read(address, Device::PPU)
    }

    fn write_bus(&mut self, address: u16, data: u8) {
        self.bus.write(address, data, Device::PPU);
    }

    fn read_sprite_byte(&self, address: u8) -> u8 {
        let sprite_location = address >> 2;
        self.primary_oam[sprite_location as usize].read_offset(address & 0b11)
    }

    fn write_sprite_byte(&mut self, address: u8, data: u8) {
        let sprite_location = address >> 2;
        self.primary_oam[sprite_location as usize].write_offset(address & 0b11, data);
    }

    // SCROLL attributes START
    fn current_coarse_x_scroll(&self) -> u8 {
        (self.vram_address_cur.get() & 0b11111) as u8
    }

    fn set_current_coarse_x_scroll(&mut self, coarse_x: u8) {
        let vram_cur = self.vram_address_cur.get_mut();

        // clear first 5 bits
        *vram_cur &= 0xFFE0;
        // copy new value
        *vram_cur |= (coarse_x & 0b11111) as u16;
    }

    fn current_coarse_y_scroll(&self) -> u8 {
        ((self.vram_address_cur.get() >> 5) & 0b11111) as u8
    }

    fn set_current_coarse_y_scroll(&mut self, coarse_y: u8) {
        let vram_cur = self.vram_address_cur.get_mut();

        // clear second 5 bits
        *vram_cur &= 0xFC1F;
        // copy new value
        *vram_cur |= ((coarse_y & 0b11111) as u16) << 5;
    }

    fn current_fine_x_scroll(&self) -> u8 {
        self.fine_x_scroll
    }

    // this is just for completion, and mostly it will not be ever used
    #[allow(unused)]
    fn set_current_fine_x_scroll(&mut self, fine_x: u8) {
        self.fine_x_scroll = fine_x & 0b111;
    }

    fn current_fine_y_scroll(&self) -> u8 {
        ((self.vram_address_cur.get() >> 12) & 0b111) as u8
    }

    fn set_current_fine_y_scroll(&mut self, fine_y: u8) {
        let vram_cur = self.vram_address_cur.get_mut();

        // clear fine_y
        *vram_cur &= 0x0FFF;
        // copy new value
        *vram_cur |= ((fine_y & 0b111) as u16) << 12;
    }

    fn top_left_coarse_x_scroll(&self) -> u8 {
        (self.vram_address_top_left & 0b11111) as u8
    }

    fn set_top_left_coarse_x_scroll(&mut self, coarse_x: u8) {
        // clear first 5 bits
        self.vram_address_top_left &= 0xFFE0;
        // copy new value
        self.vram_address_top_left |= (coarse_x & 0b11111) as u16;
    }

    fn top_left_coarse_y_scroll(&self) -> u8 {
        ((self.vram_address_top_left >> 5) & 0b11111) as u8
    }

    fn set_top_left_coarse_y_scroll(&mut self, coarse_y: u8) {
        // clear second 5 bits
        self.vram_address_top_left &= 0xFC1F;
        // copy new value
        self.vram_address_top_left |= ((coarse_y & 0b11111) as u16) << 5;
    }

    fn set_top_left_fine_x_scroll(&mut self, fine_x: u8) {
        self.fine_x_scroll = fine_x & 0b111;
    }

    fn top_left_fine_y_scroll(&self) -> u8 {
        ((self.vram_address_top_left >> 12) & 0b111) as u8
    }

    fn set_top_left_fine_y_scroll(&mut self, fine_y: u8) {
        // clear fine_y
        self.vram_address_top_left &= 0x0FFF;
        // copy new value
        self.vram_address_top_left |= ((fine_y & 0b111) as u16) << 12;
    }

    fn set_top_left_x_scroll(&mut self, x_scroll: u8) {
        self.set_top_left_coarse_x_scroll(x_scroll >> 3);
        self.set_top_left_fine_x_scroll(x_scroll & 0b111);
    }

    fn set_top_left_y_scroll(&mut self, y_scroll: u8) {
        self.set_top_left_coarse_y_scroll(y_scroll >> 3);
        self.set_top_left_fine_y_scroll(y_scroll & 0b111);
    }

    fn increment_y_scroll(&mut self) {
        // increment fine scrolling Y on the last dot without carry
        let fine_y = self.current_fine_y_scroll() + 1;

        self.set_current_fine_y_scroll(fine_y & 0b111);

        // if the increment resulted in a carry, go to the next tile
        // i.e. increment coarse Y
        if fine_y & 0x8 != 0 {
            // extract coarse_y
            let mut coarse_y = self.current_coarse_y_scroll();

            // in case of overflow, increment nametable vertical address
            if coarse_y == 29 {
                coarse_y = 0;
                self.increment_vram_nametable_vertical();
            } else if coarse_y == 31 {
                coarse_y = 0;
            } else {
                coarse_y += 1;
            }

            self.set_current_coarse_y_scroll(coarse_y);
        }
    }

    fn increment_coarse_x_scroll(&mut self) {
        let coarse_x = self.current_coarse_x_scroll() + 1;

        self.set_current_coarse_x_scroll(coarse_x & 0b11111);

        // in case of overflow, increment nametable horizontal address
        if coarse_x & 0b100000 != 0 {
            self.increment_vram_nametable_horizontal();
        }
    }
    // SCROLL attributes END

    fn increment_vram_readwrite(&self) {
        // only increment if its valid, and increment by the correct ammount
        if self.scanline > 240 || !self.reg_mask.rendering_enabled() {
            self.vram_address_cur
                .set(self.vram_address_cur.get() + self.reg_control.vram_increment());

            // dummy read to update the cartridge, which mappers rely on some
            // address pins from the PPU
            let _ = self.read_bus(self.vram_address_cur.get());
        }
    }

    fn restore_rendering_scroll_x(&mut self) {
        self.set_current_coarse_x_scroll(self.top_left_coarse_x_scroll());
    }

    fn restore_rendering_scroll_y(&mut self) {
        self.set_current_fine_y_scroll(self.top_left_fine_y_scroll());
        self.set_current_coarse_y_scroll(self.top_left_coarse_y_scroll());
    }

    fn increment_vram_nametable_horizontal(&mut self) {
        *self.vram_address_cur.get_mut() ^= 0b01 << 10;
    }

    fn increment_vram_nametable_vertical(&mut self) {
        *self.vram_address_cur.get_mut() ^= 0b10 << 10;
    }

    // restore from top_left or original nametable selector from `reg_control`
    fn restore_nametable(&mut self) {
        let vram_cur = self.vram_address_cur.get_mut();

        // clear old nametable data
        *vram_cur &= 0xF3FF;

        *vram_cur |= (self.reg_control.nametable_selector() as u16) << 10;
    }

    fn restore_nametable_horizontal(&mut self) {
        let vram_cur = self.vram_address_cur.get_mut();

        // clear horizontal nametable data
        *vram_cur &= 0xFBFF;

        *vram_cur |= (self.reg_control.nametable_selector() as u16 & 1) << 10;
    }

    fn current_nametable(&self) -> u16 {
        (self.vram_address_cur.get() >> 10) & 0b11
    }

    // this should only be called when rendering and a bit after that,
    // i.e. when scanline number is in range 0 >= scanline > 255
    fn get_next_scroll_y_render(&self) -> u8 {
        if self.scanline == 261 {
            0
        } else if self.scanline < 255 {
            (self.scanline + 1) as u8
        } else {
            unreachable!()
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn reload_background_shift_registers(&mut self) {
        // tile address = 0x2000 | (v & 0x0FFF)
        let nametable_tile = self.read_bus(0x2000 | self.vram_address_cur.get() & 0xFFF);

        let tile_pattern = self.fetch_pattern_background(nametable_tile);

        // fetch and prepare the palette
        let attribute_byte = self.fetch_attribute_byte();

        // Each byte controls the palette of a 32×32 pixel or 4×4 tile part of the
        // nametable and is divided into four 2-bit areas. Each area covers 16×16
        // pixels or 2×2 tiles. Given palette numbers topleft, topright,
        // bottomleft, bottomright, each in the range 0 to 3, the value of
        // the byte is
        // `value = (bottomright << 6) | (bottomleft << 4) | (topright << 2) | (topleft << 0)`
        let coarse_x = self.current_coarse_x_scroll();
        let coarse_y = self.current_coarse_y_scroll();
        let attribute_location_x = (coarse_x >> 1) & 0x1;
        let attribute_location_y = (coarse_y >> 1) & 0x1;

        // `attribute_location_x`: 0 => left, 1 => right
        // `attribute_location_y`: 0 => top, 1 => bottom
        let attribute_location = attribute_location_y << 1 | attribute_location_x;

        // 00: top-left, 01: top-right, 10: bottom-left, 11: bottom-right
        // bit-1 is for (top, bottom), bit-0 is for (left, right)
        let palette = (attribute_byte >> (attribute_location * 2)) & 0b11;

        // update th shift registers
        for i in 0..=1 {
            // clear the bottom value
            self.bg_pattern_shift_registers[i] &= 0xFF00;

            // in this stage, because we reload in dots (8, 16, 24...)
            // the shift registers will be shifted one more time
            // meaning, it will be shifted 8 times
            self.bg_pattern_shift_registers[i] |= tile_pattern[i] as u16;

            // clear the bottom value
            self.bg_palette_shift_registers[i] &= 0xFF00;

            // as palettes are two bits, we store the first bit in index 0 and
            // the second bit in index 1 in the array
            //
            // this is similar to how the patterns are stored in CHR table
            self.bg_palette_shift_registers[i] |= 0xFF * ((palette >> i) & 1) as u16;
        }
    }

    /// ## PPU pattern table addressing ##
    /// DCBA98 76543210
    /// ---------------
    /// 0HRRRR CCCCPTTT
    /// |||||| |||||+++- T: Fine Y offset, the row number within a tile
    /// |||||| ||||+---- P: Bit plane (0: "lower"; 1: "upper")
    /// |||||| ++++----- C: Tile column
    /// ||++++---------- R: Tile row
    /// |+-------------- H: Half of sprite table (0: "left"; 1: "right")
    /// +--------------- 0: Pattern table is at $0000-$1FFF
    #[allow(clippy::identity_op)]
    fn fetch_pattern(&self, pattern_table: u16, location: u8, fine_y: u8) -> [u8; 2] {
        let fine_y = fine_y as u16;

        let low_plane_pattern =
            self.read_bus(pattern_table | (location as u16) << 4 | 0 << 3 | fine_y);

        let high_plane_pattern =
            self.read_bus(pattern_table | (location as u16) << 4 | 1 << 3 | fine_y);

        [low_plane_pattern, high_plane_pattern]
    }

    fn fetch_pattern_background(&self, location: u8) -> [u8; 2] {
        let fine_y = self.current_fine_y_scroll();

        // for background
        let pattern_table = self.reg_control.background_pattern_address();

        self.fetch_pattern(pattern_table, location, fine_y)
    }

    /// ## Attribute address ##
    /// NN 1111 YYY XXX
    /// || |||| ||| +++-- high 3 bits of coarse X (x/4)
    /// || |||| +++------ high 3 bits of coarse Y (y/4)
    /// || ++++---------- attribute offset (960 bytes)
    /// ++--------------- nametable select
    ///
    /// or
    ///
    /// `attribute address = 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07)`
    /// where x, y and nametable are used from `vram_address_cur`
    fn fetch_attribute_byte(&self) -> u8 {
        let x = (self.current_coarse_x_scroll() >> 2) as u16;
        let y = (self.current_coarse_y_scroll() >> 2) as u16;

        self.read_bus(0x2000 | self.current_nametable() << 10 | 0xF << 6 | y << 3 | x)
    }

    fn reload_sprite_shift_registers(&mut self) {
        // move sprite_0_present
        self.sprite_0_present = self.next_scanline_sprite_0_present;
        // reset for scanline after next
        self.next_scanline_sprite_0_present = false;

        let next_y = self.get_next_scroll_y_render();

        // loop through all secondary_oam, even the empty ones (0xFF)
        // a write to the cartridge MUST be done here even if no sprites
        // are drawn
        for i in 0..8 as usize {
            let sprite = self.secondary_oam[i];
            let mut fine_y = next_y.wrapping_sub(sprite.get_y());

            let sprite_height = self.reg_control.sprite_height();

            // handle flipping vertically
            if sprite.get_attribute().is_flip_vertical() {
                fine_y = (sprite_height - 1).wrapping_sub(fine_y);
            }

            self.sprite_counters[i] = sprite.get_x();
            self.sprite_pattern_shift_registers[i] =
                self.fetch_pattern_sprite(sprite.get_tile(), fine_y);

            // handle flipping horizontally
            if sprite.get_attribute().is_flip_horizontal() {
                let mut tmp_low = 0;
                let mut tmp_high = 0;

                // this whole loop and the bit after it is just to rotate the bits
                // of the `sprite_pattern_shift_registers`, such that bits
                // `0,1,2,...,n` would become bits `n,...,2,1,0`
                //
                // it does not look very efficient to me, but not sure if there
                // is a faster method
                for _ in 0..7 {
                    tmp_low |= self.sprite_pattern_shift_registers[i][0] & 0b1;
                    tmp_high |= self.sprite_pattern_shift_registers[i][1] & 0b1;

                    tmp_low <<= 1;
                    tmp_high <<= 1;
                    self.sprite_pattern_shift_registers[i][0] >>= 1;
                    self.sprite_pattern_shift_registers[i][1] >>= 1;
                }

                // put the reminaing bit without shifting
                tmp_low |= self.sprite_pattern_shift_registers[i][0];
                tmp_high |= self.sprite_pattern_shift_registers[i][1];

                self.sprite_pattern_shift_registers[i][0] = tmp_low;
                self.sprite_pattern_shift_registers[i][1] = tmp_high;
            }

            self.sprite_attribute_registers[i] = sprite.get_attribute();
        }
    }

    fn fetch_pattern_sprite(&self, tile: u8, mut fine_y: u8) -> [u8; 2] {
        let mut location = tile;

        // for sprites
        let pattern_table = if self.reg_control.sprite_height() == 16 {
            // zero the first bit as it is used as a pattern_table selector
            location &= !(1);
            ((tile & 1) as u16) << 12
        } else {
            self.reg_control.sprite_pattern_address()
        };

        if fine_y > 7 {
            fine_y -= 8;
            location = location.wrapping_add(1);
        }

        self.fetch_pattern(pattern_table, location, fine_y)
    }

    fn get_background_pixel(&self) -> (u8, u8) {
        let fine_x = self.current_fine_x_scroll();

        // select the bit using `fine_x` from the left
        let bit_location = 15 - fine_x;

        let low_plane_bit =
            ((self.bg_pattern_shift_registers[0] >> bit_location as u16) & 0x1) as u8;
        let high_plane_bit =
            ((self.bg_pattern_shift_registers[1] >> bit_location as u16) & 0x1) as u8;

        let color_bit = high_plane_bit << 1 | low_plane_bit;

        let low_palette_bit =
            ((self.bg_palette_shift_registers[0] >> bit_location as u16) & 0x1) as u8;
        let high_palette_bit =
            ((self.bg_palette_shift_registers[1] >> bit_location as u16) & 0x1) as u8;

        let palette = high_palette_bit << 1 | low_palette_bit;

        let background_enabled = self.reg_mask.background_enabled() as u8;
        // if background is not enabled, it will be multiplied by zero which is zero
        (color_bit * background_enabled, palette * background_enabled)
    }

    fn get_sprites_first_non_transparent_pixel(&mut self) -> (u8, u8, bool, bool) {
        let mut color_bits = 0;
        let mut palette = 0;
        let mut background_priority = false;
        let mut first_non_transparent_found = false;
        let mut is_sprite_0 = false;

        for i in 0..8 {
            // active sprite
            if self.sprite_counters[i] == 0 {
                // the color bit
                let low_bit = self.sprite_pattern_shift_registers[i][0] >> 7;
                let high_bit = self.sprite_pattern_shift_registers[i][1] >> 7;

                // shift the registers
                self.sprite_pattern_shift_registers[i][0] =
                    self.sprite_pattern_shift_registers[i][0].wrapping_shl(1);
                self.sprite_pattern_shift_registers[i][1] =
                    self.sprite_pattern_shift_registers[i][1].wrapping_shl(1);

                let current_color_bits = (high_bit << 1) | low_bit;

                // if its a zero, ignore it and try to find the next non-transparent
                // color-bit, if all are zeros, then ok
                if !first_non_transparent_found && current_color_bits != 0 {
                    color_bits = current_color_bits;

                    let attribute = self.sprite_attribute_registers[i];
                    palette = attribute.palette();
                    background_priority = attribute.is_behind_background();

                    // set if its sprite 0
                    is_sprite_0 = i == 0 && self.sprite_0_present;

                    // stop searching
                    first_non_transparent_found = true;
                }
            } else {
                self.sprite_counters[i] -= 1;
            }
        }

        let sprites_enabled = self.reg_mask.sprites_enabled();
        // if sprites is not enabled, this will be (0, 0, false, false)
        (
            color_bits * sprites_enabled as u8,
            palette * sprites_enabled as u8,
            background_priority && sprites_enabled,
            is_sprite_0 && sprites_enabled,
        )
    }

    /// this method fetches background and sprite pixels, check overflow for
    /// sprite_0 and priority, and handles the left 8-pixel clipping
    /// and then outputs a color index
    ///
    /// ## color location offset 0x3F00 ##
    /// 43210
    /// |||||
    /// |||++- Pixel value from tile data
    /// |++--- Palette number from attribute table or OAM
    /// +----- Background/Sprite select
    fn generate_pixel(&mut self) -> u8 {
        // fetch the next background pixel (it must fetch to advance the
        // shift registers), and then decide if we should clip or not
        let background_pixel_data = self.get_background_pixel();
        let (background_color_bits, background_palette) =
            if self.cycle < 8 && self.reg_mask.background_left_clipping_enabled() {
                (0, 0)
            } else {
                background_pixel_data
            };

        // fetch the next sprite pixel (it must fetch to advance the
        // shift registers), and then decide if we should clip or not
        let sprite_pixel_data = self.get_sprites_first_non_transparent_pixel();

        // another special case is when x is 255, the sprite should always miss
        let (sprite_color_bits, sprite_palette, background_priority, is_sprite_0) =
            if self.cycle < 8 && self.reg_mask.sprites_left_clipping_enabled() || self.cycle == 255
            {
                // since the pixel data is `0`, the other data (palette, priority, ..)
                // are not important
                (0, 0, false, false)
            } else {
                sprite_pixel_data
            };

        // 0 for background, 1 for sprite
        let palette_selector;
        // palette index
        let mut palette;
        let color_bits;

        // sprite and background multiplexer procedure
        if sprite_color_bits != 0 && background_color_bits != 0 {
            // use background priority flag
            if background_priority {
                color_bits = background_color_bits;
                palette = background_palette;
                palette_selector = 0;
            } else {
                color_bits = sprite_color_bits;
                palette = sprite_palette;
                palette_selector = 1;
            }
            if is_sprite_0 {
                // if sprite and background are not transparent, then there is a collision
                self.reg_status.get_mut().insert(StatusReg::SPRITE_0_HIT);
            }
        } else if sprite_color_bits != 0 {
            color_bits = sprite_color_bits;
            palette = sprite_palette;
            palette_selector = 1;
        } else {
            color_bits = background_color_bits;
            palette = background_palette;
            palette_selector = 0;
        }

        if color_bits == 0 {
            // universal background color
            palette = 0;
        }

        let color =
            self.read_bus(0x3F00 | (palette_selector << 4 | palette << 2 | color_bits) as u16);

        // advance the shift registers
        for i in 0..=1 {
            self.bg_pattern_shift_registers[i] = self.bg_pattern_shift_registers[i].wrapping_shl(1);
            self.bg_palette_shift_registers[i] = self.bg_palette_shift_registers[i].wrapping_shl(1);
        }

        color
    }

    fn emphasis_color(&self, color: Color) -> Color {
        let is_red_emph = self.reg_mask.intersects(MaskReg::EMPHASIZE_RED);
        let is_green_emph = self.reg_mask.intersects(MaskReg::EMPHASIZE_GREEN);
        let is_blue_emph = self.reg_mask.intersects(MaskReg::EMPHASIZE_BLUE);

        let mut red = 1.;
        let mut green = 1.;
        let mut blue = 1.;

        if is_red_emph {
            red *= 1.1;
            green *= 0.9;
            blue *= 0.9;
        }
        if is_green_emph {
            red *= 0.9;
            green *= 1.1;
            blue *= 0.9;
        }
        if is_blue_emph {
            red *= 0.9;
            green *= 0.9;
            blue *= 1.1;
        }

        Color {
            r: min((color.r as f32 * red) as u8, 255),
            g: min((color.g as f32 * green) as u8, 255),
            b: min((color.b as f32 * blue) as u8, 255),
        }
    }

    fn render_pixel(&mut self) {
        // fix overflowing colors
        let mut color = self.generate_pixel() & 0x3F;

        if self.reg_mask.is_grayscale() {
            // select from the gray column (0x00, 0x10, 0x20, 0x30)
            color &= 0x30;
        }

        // render the color
        self.tv.set_pixel(
            self.cycle as u32,
            self.scanline as u32,
            &self.emphasis_color(COLORS[color as usize]),
        );
    }

    // run one cycle, this should be fed from Master clock
    pub fn clock(&mut self) {
        // current scanline
        match self.scanline {
            261 => {
                // pre-render

                match self.cycle {
                    0 => {
                        // FIXME: for some reason the test only worked when doing it here

                        // clear sprite 0 hit
                        self.reg_status.get_mut().remove(StatusReg::SPRITE_0_HIT)
                    }
                    2 => {
                        // reset nmi_occured_in_this_frame
                        self.nmi_occured_in_this_frame.set(false);
                    }
                    1 => {
                        // clear sprite overflow
                        self.reg_status.get_mut().remove(StatusReg::SPRITE_OVERFLOW);
                        // clear v-blank
                        self.reg_status.get_mut().remove(StatusReg::VERTICAL_BLANK);

                        if self.reg_mask.rendering_enabled() {
                            self.restore_rendering_scroll_x();
                            self.restore_rendering_scroll_y();

                            self.restore_nametable();

                            // load next 2 bytes
                            for _ in 0..2 {
                                for i in 0..=1 {
                                    // as this is the first time, shift the registers
                                    // as we are reloading 2 times
                                    self.bg_pattern_shift_registers[i] =
                                        self.bg_pattern_shift_registers[i].wrapping_shl(8);
                                    self.bg_palette_shift_registers[i] =
                                        self.bg_palette_shift_registers[i].wrapping_shl(8);
                                }
                                self.reload_background_shift_registers();
                                self.increment_coarse_x_scroll();
                            }
                        }
                    }

                    257 => {
                        // reload all of them in one go
                        self.reload_sprite_shift_registers();
                    }
                    _ => {}
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
                    if self.reg_control.nmi_enabled() && !self.nmi_occured_in_this_frame.get() {
                        self.nmi_pin_status.set(true);
                        self.nmi_occured_in_this_frame.set(true);
                    }
                }
            }
            _ => {
                unreachable!();
            }
        }
        self.cycle += 1;
        if self.cycle > 340
            || (self.scanline == 261
                && self.cycle == 340
                && self.is_odd_frame
                && self.reg_mask.rendering_enabled())
        {
            self.scanline += 1;
            self.cycle = 0;

            // next frame
            if self.scanline > 261 {
                self.scanline = 0;
                self.is_odd_frame = !self.is_odd_frame;
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
                    self.reload_background_shift_registers();

                    if self.cycle != 256 {
                        // increment scrolling X in current VRAM address
                        self.increment_coarse_x_scroll();
                    } else {
                        // fine and carry to coarse
                        self.increment_y_scroll();
                    }
                }

                // secondary OAM clear, cycles 1-64, but we do it in one go
                // TODO: should it be in multiple times, instead of one go?
                if self.cycle == 1 {
                    self.secondary_oam = [Sprite::filled_ff(); 8];

                    // reset counter
                    self.secondary_oam_counter = 0;
                }

                // 65 - 256
                if self.cycle >= 65 && self.cycle <= 256 {
                    let next_y = self.get_next_scroll_y_render() as i16;

                    let mut index = (self.cycle - 65) as usize;
                    // each takes 3 cycles
                    if index % 3 == 0 {
                        index /= 3;

                        let sprite = self.primary_oam[index];
                        let sprite_y = sprite.get_y() as i16;

                        let diff = next_y - sprite_y;
                        let height = self.reg_control.sprite_height() as i16;

                        if diff >= 0 && diff < height {
                            // in range

                            // sprite 0
                            if index == 0 {
                                self.next_scanline_sprite_0_present = true;
                            }

                            if self.secondary_oam_counter > 7 {
                                // overflow
                                self.reg_status.get_mut().insert(StatusReg::SPRITE_OVERFLOW);
                            } else {
                                self.secondary_oam[self.secondary_oam_counter as usize] = sprite;

                                self.secondary_oam_counter += 1;
                            }
                        }
                    }
                }
            }
            257..=320 => {
                // unused
                if self.cycle == 257 {
                    self.restore_rendering_scroll_x();
                    // to fix nametable wrapping around
                    self.restore_nametable_horizontal();
                }

                // reload them all in one go
                if self.cycle == 257 {
                    self.reload_sprite_shift_registers();
                }
            }
            321..=340 => {
                // lets just do it in the beginning
                if self.cycle == 321 {
                    // load next 2 bytes
                    for _ in 0..2 {
                        for i in 0..=1 {
                            // as this is the first time, shift the registers
                            // as we are reloading 2 times
                            self.bg_pattern_shift_registers[i] =
                                self.bg_pattern_shift_registers[i].wrapping_shl(8);
                            self.bg_palette_shift_registers[i] =
                                self.bg_palette_shift_registers[i].wrapping_shl(8);
                        }
                        self.reload_background_shift_registers();
                        self.increment_coarse_x_scroll();
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

    pub fn reset(&mut self, bus: T) {
        // just as if calling the constructor but without TV, just reset it
        self.reg_control = ControlReg::empty();
        self.reg_mask = MaskReg::empty();
        self.reg_status = Cell::new(StatusReg::empty());
        self.reg_oam_addr = Cell::new(0);

        self.scanline = 0; // start from -1 scanline
        self.cycle = 0;

        self.vram_address_cur = Cell::new(0);
        self.vram_address_top_left = 0;

        self.ppu_data_read_buffer = Cell::new(0);

        self.fine_x_scroll = 0;

        self.w_toggle = Cell::new(false);

        self.bg_pattern_shift_registers = [0; 2];
        self.bg_palette_shift_registers = [0; 2];

        self.nmi_pin_status = Cell::new(false);
        self.nmi_occured_in_this_frame = Cell::new(false);

        self.bus = bus;

        self.primary_oam = [Sprite::empty(); 64];
        self.secondary_oam = [Sprite::empty(); 8];

        self.secondary_oam_counter = 0;

        self.sprite_pattern_shift_registers = [[0; 2]; 8];
        self.sprite_attribute_registers = [SpriteAttribute::empty(); 8];
        self.sprite_counters = [0; 8];
        self.sprite_0_present = false;
        self.next_scanline_sprite_0_present = false;

        self.is_dma_request = false;
        self.dma_request_address = 0;

        self.is_odd_frame = false;

        self.tv.reset();
    }

    fn load_serialized_state(&mut self, state: SavablePPUState) {
        let mut primary_oam = [Sprite::empty(); 64];
        primary_oam.copy_from_slice(state.primary_oam.as_slice());

        self.reg_control = ControlReg::from_bits(state.reg_control).unwrap();
        self.reg_mask = MaskReg::from_bits(state.reg_mask).unwrap();
        *self.reg_status.get_mut() = StatusReg::from_bits(state.reg_status).unwrap();
        *self.reg_oam_addr.get_mut() = state.reg_oam_addr;
        self.scanline = state.scanline;
        self.cycle = state.cycle;
        *self.vram_address_cur.get_mut() = state.vram_address_cur;
        self.vram_address_top_left = state.vram_address_top_left;
        *self.ppu_data_read_buffer.get_mut() = state.ppu_data_read_buffer;
        self.fine_x_scroll = state.fine_x_scroll;
        *self.w_toggle.get_mut() = state.w_toggle;
        self.bg_pattern_shift_registers = state.bg_pattern_shift_registers;
        self.bg_palette_shift_registers = state.bg_palette_shift_registers;
        *self.nmi_pin_status.get_mut() = state.nmi_pin_status;
        *self.nmi_occured_in_this_frame.get_mut() = state.nmi_occured_in_this_frame;
        self.primary_oam = primary_oam;
        self.secondary_oam = state.secondary_oam;
        self.secondary_oam_counter = state.secondary_oam_counter;
        self.sprite_pattern_shift_registers = state.sprite_pattern_shift_registers;
        self.sprite_attribute_registers = state.sprite_attribute_registers;
        self.sprite_counters = state.sprite_counters;
        self.sprite_0_present = state.sprite_0_present;
        self.next_scanline_sprite_0_present = state.next_scanline_sprite_0_present;
        self.is_dma_request = state.is_dma_request;
        self.dma_request_address = state.dma_request_address;
        self.is_odd_frame = state.is_odd_frame;
    }
}

impl<T> PPUCPUConnection for PPU2C02<T>
where
    T: Bus + Savable,
{
    fn is_nmi_pin_set(&self) -> bool {
        self.nmi_pin_status.get()
    }

    fn clear_nmi_pin(&mut self) {
        self.nmi_pin_status.set(false);
    }

    fn is_dma_request(&self) -> bool {
        self.is_dma_request
    }

    fn clear_dma_request(&mut self) {
        self.is_dma_request = false;
    }

    fn dma_address(&mut self) -> u8 {
        self.dma_request_address
    }

    fn send_oam_data(&mut self, address: u8, data: u8) {
        self.write_sprite_byte(self.reg_oam_addr.get().wrapping_add(address), data);
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SavablePPUState {
    reg_control: u8,
    reg_mask: u8,
    reg_status: u8,
    reg_oam_addr: u8,

    scanline: u16,
    cycle: u16,

    vram_address_cur: u16,
    vram_address_top_left: u16,

    ppu_data_read_buffer: u8,

    fine_x_scroll: u8,

    w_toggle: bool,

    bg_pattern_shift_registers: [u16; 2],
    bg_palette_shift_registers: [u16; 2],

    nmi_pin_status: bool,
    nmi_occured_in_this_frame: bool,

    primary_oam: Vec<Sprite>,
    secondary_oam: [Sprite; 8],

    secondary_oam_counter: u8,

    sprite_pattern_shift_registers: [[u8; 2]; 8],
    sprite_attribute_registers: [SpriteAttribute; 8],
    sprite_counters: [u8; 8],
    sprite_0_present: bool,
    next_scanline_sprite_0_present: bool,

    is_dma_request: bool,
    dma_request_address: u8,

    is_odd_frame: bool,
}

impl SavablePPUState {
    fn from_ppu<T: Bus + Savable>(ppu: &PPU2C02<T>) -> Self {
        let mut primary_oam = Vec::with_capacity(ppu.primary_oam.len());
        primary_oam.extend_from_slice(&ppu.primary_oam);

        Self {
            reg_control: ppu.reg_control.bits(),
            reg_mask: ppu.reg_mask.bits(),
            reg_status: ppu.reg_status.get().bits(),
            reg_oam_addr: ppu.reg_oam_addr.get(),
            scanline: ppu.scanline,
            cycle: ppu.cycle,
            vram_address_cur: ppu.vram_address_cur.get(),
            vram_address_top_left: ppu.vram_address_top_left,
            ppu_data_read_buffer: ppu.ppu_data_read_buffer.get(),
            fine_x_scroll: ppu.fine_x_scroll,
            w_toggle: ppu.w_toggle.get(),
            bg_pattern_shift_registers: ppu.bg_pattern_shift_registers,
            bg_palette_shift_registers: ppu.bg_palette_shift_registers,
            nmi_pin_status: ppu.nmi_pin_status.get(),
            nmi_occured_in_this_frame: ppu.nmi_occured_in_this_frame.get(),
            primary_oam,
            secondary_oam: ppu.secondary_oam,
            secondary_oam_counter: ppu.secondary_oam_counter,
            sprite_pattern_shift_registers: ppu.sprite_pattern_shift_registers,
            sprite_attribute_registers: ppu.sprite_attribute_registers,
            sprite_counters: ppu.sprite_counters,
            sprite_0_present: ppu.sprite_0_present,
            next_scanline_sprite_0_present: ppu.next_scanline_sprite_0_present,
            is_dma_request: ppu.is_dma_request,
            dma_request_address: ppu.dma_request_address,
            is_odd_frame: ppu.is_odd_frame,
        }
    }
}

impl<T: Bus + Savable> Savable for PPU2C02<T> {
    fn save<W: std::io::Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        self.bus.save(writer)?;

        let state = SavablePPUState::from_ppu(self);

        bincode::serialize_into(writer, &state).map_err(|err| match *err {
            bincode::ErrorKind::Io(err) => SaveError::IoError(err),
            _ => SaveError::Others,
        })?;

        Ok(())
    }

    fn load<R: std::io::Read>(&mut self, reader: &mut R) -> Result<(), SaveError> {
        self.bus.load(reader)?;

        let state: SavablePPUState =
            bincode::deserialize_from(reader).map_err(|err| match *err {
                bincode::ErrorKind::Io(err) => SaveError::IoError(err),
                _ => SaveError::Others,
            })?;

        self.load_serialized_state(state);

        Ok(())
    }
}
