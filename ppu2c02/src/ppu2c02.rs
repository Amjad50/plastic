use crate::ppu2c02_registers::Register;
use crate::sprite::{Sprite, SpriteAttribute};
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

pub struct PPU2C02<T: Bus> {
    // memory mapped registers
    reg_control: ControlReg,
    reg_mask: MaskReg,
    reg_status: Cell<StatusReg>,
    reg_oam_addr: Cell<u8>,

    scanline: u16,
    cycle: u16,

    vram_address_cur: Cell<u16>,
    vram_address_top_left: u16,

    ppu_data_read_buffer: Cell<u8>,

    fine_x_scroll: u8,

    w_toggle: Cell<bool>, // this is used for registers that require 2 writes

    bg_pattern_shift_registers: [u16; 2],
    bg_palette_shift_registers: [u16; 2],

    nmi_pin_status: bool,
    nmi_occured_in_this_frame: bool,

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
    T: Bus,
{
    pub fn new(bus: T, tv: TV) -> Self {
        Self {
            reg_control: ControlReg::empty(),
            reg_mask: MaskReg::empty(),
            reg_status: Cell::new(StatusReg::empty()),
            reg_oam_addr: Cell::new(0),

            scanline: 261, // start from -1 scanline
            cycle: 0,

            vram_address_cur: Cell::new(0),
            vram_address_top_left: 0,

            ppu_data_read_buffer: Cell::new(0),

            fine_x_scroll: 0,

            w_toggle: Cell::new(false),

            bg_pattern_shift_registers: [0; 2],
            bg_palette_shift_registers: [0; 2],

            nmi_pin_status: false,
            nmi_occured_in_this_frame: false,

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
                        && !self.nmi_occured_in_this_frame
                    {
                        self.nmi_pin_status = true;
                        self.nmi_occured_in_this_frame = true;
                    }
                } else {
                    // in case if the NMI flag was disabled, then mark as nmi
                    // never occurred on this frame, even if it has
                    // meaning, that in some cases 2 NMI can occur
                    self.nmi_occured_in_this_frame = false;
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

    fn reload_shift_registers(&mut self) {
        let nametable_tile = self.read_bus(0x2000 | self.vram_address_cur.get() & 0xFFF);

        let tile_pattern = self.fetch_pattern_background(nametable_tile);

        // fetch and prepare the palette
        let attribute_byte = self.fetch_attribute_byte();

        let coarse_x = self.current_coarse_x_scroll();
        let coarse_y = self.current_coarse_y_scroll();

        let attribute_location_x = (coarse_x >> 1) & 0x1;
        let attribute_location_y = (coarse_y >> 1) & 0x1;

        let attribute_location = attribute_location_y << 1 | attribute_location_x;

        // 00: top-left, 01: top-right, 10: bottom-left, 11: bottom-right
        // bit-1 is for (top, bottom), bit-0 is for (left, right)
        let palette = (attribute_byte >> (attribute_location * 2)) & 0b11;

        // update th shift registers
        for i in 0..=1 {
            self.bg_pattern_shift_registers[i] &= 0xFF00;

            // in this stage, because we reload in dots (8, 16, 24...)
            // the shift registers will be shifted one more time
            // meaning, it will be shifted 8 times
            self.bg_pattern_shift_registers[i] |= tile_pattern[i] as u16;

            // palette
            self.bg_palette_shift_registers[i] &= 0xFF00;

            // as palettes are two bits, we store the first bit in index 0 and
            // the second bit in index 1 in the array
            //
            // this is similar to how the patterns are stored in CHR table
            self.bg_palette_shift_registers[i] |= 0xFF * ((palette >> i) & 1) as u16;
        }
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
        let fine_y = self.current_fine_y_scroll() as u16;

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

        // must not exceed 8
        assert!(self.secondary_oam_counter <= 8);

        for i in 0..self.secondary_oam_counter as usize {
            let sprite = self.secondary_oam[i];
            let mut fine_y = next_y.wrapping_sub(sprite.get_y());

            // handle flipping vertically
            if sprite.get_attribute().is_flip_vertical() {
                fine_y = 7 - fine_y;
            }

            self.sprite_counters[i] = sprite.get_x();
            self.sprite_pattern_shift_registers[i] =
                self.fetch_pattern_sprite(sprite.get_tile(), fine_y);

            // handle flipping horizontally
            if sprite.get_attribute().is_flip_horizontal() {
                let mut tmp_low = 0;
                let mut tmp_high = 0;
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
        // fill the remianing bytes with empty patterns, x and attributes
        // should be equal to 0xFF
        for i in self.secondary_oam_counter as usize..8 {
            let sprite = self.secondary_oam[i];
            self.sprite_counters[i] = sprite.get_x();
            // empty shift registers
            self.sprite_pattern_shift_registers[i] = [0; 2];
            self.sprite_attribute_registers[i] = sprite.get_attribute();
        }
    }

    fn fetch_pattern_sprite(&self, location: u8, fine_y: u8) -> [u8; 2] {
        // for sprites
        let pattern_table = self.reg_control.sprite_pattern_address();

        let low_plane_pattern =
            self.read_bus(pattern_table | (location as u16) << 4 | 0 << 3 | fine_y as u16);

        let high_plane_pattern =
            self.read_bus(pattern_table | (location as u16) << 4 | 1 << 3 | fine_y as u16);

        [low_plane_pattern, high_plane_pattern]
    }

    fn get_background_pixel(&self) -> (u8, u8) {
        // skip all this, if the background is disabled
        if !self.reg_mask.background_enabled() {
            return (0, 0);
        }

        let fine_x = self.current_fine_x_scroll();
        let low_plane_bit =
            ((self.bg_pattern_shift_registers[0] >> (15 - fine_x) as u16) & 0x1) as u8;
        let high_plane_bit =
            ((self.bg_pattern_shift_registers[1] >> (15 - fine_x) as u16) & 0x1) as u8;

        let color_bit = high_plane_bit << 1 | low_plane_bit;

        let low_palette_bit =
            ((self.bg_palette_shift_registers[0] >> (15 - fine_x) as u16) & 0x1) as u8;
        let high_palette_bit =
            ((self.bg_palette_shift_registers[1] >> (15 - fine_x) as u16) & 0x1) as u8;

        let palette = high_palette_bit << 1 | low_palette_bit;

        (color_bit, palette)
    }

    fn get_sprites_first_non_transparent_pixel(&mut self) -> (u8, u8, bool, bool) {
        // skip all this, if the sprites is disabled
        if !self.reg_mask.sprites_enabled() {
            return (0, 0, false, false);
        }

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

        (color_bits, palette, background_priority, is_sprite_0)
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
        let (background_color_bits, background_palette) = self.get_background_pixel();
        let (sprite_color_bits, sprite_palette, background_priority, is_sprite_0) =
            self.get_sprites_first_non_transparent_pixel();

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

    fn render_pixel(&mut self) {
        let mut color = self.get_pixel();

        if self.reg_mask.is_grayscale() {
            // select from the gray column (0x00, 0x10, 0x20, 0x30)
            color &= 0x30;
        }

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
                    // reset nmi_occured_in_this_frame
                    self.nmi_occured_in_this_frame = false;
                    // clear v-blank
                    self.reg_status.get_mut().remove(StatusReg::VERTICAL_BLANK);
                    // clear sprite overflow
                    self.reg_status.get_mut().remove(StatusReg::SPRITE_OVERFLOW);
                    // clear sprite 0 hit
                    self.reg_status.get_mut().remove(StatusReg::SPRITE_0_HIT);

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
                            self.reload_shift_registers();
                            self.increment_coarse_x_scroll();
                        }
                    }
                }
                // reload all of them in one go
                if self.cycle == 257 && self.reg_mask.sprites_enabled() {
                    self.reload_sprite_shift_registers();
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
                        self.nmi_occured_in_this_frame = true;
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
                    self.reload_shift_registers();

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
                match self.cycle {
                    // only the first 64 cycles
                    65..=128 => {
                        let next_y = self.get_next_scroll_y_render() as i16;

                        let index = (self.cycle - 65) as usize;

                        let sprite = self.primary_oam[index];
                        let sprite_y = sprite.get_y() as i16;

                        let diff = next_y - sprite_y;
                        if diff >= 0 && diff < 8 {
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
                    _ => {}
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
                        self.reload_shift_registers();
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
