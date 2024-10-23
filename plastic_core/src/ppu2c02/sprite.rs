use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
   #[derive(Serialize, Deserialize)]
   pub struct SpriteAttribute: u8 {
       const PALETTE = 0b00000011;
       const PRIORITY = 0b00100000;
       const FLIP_HORIZONTALLY = 0b01000000;
       const FLIP_VERTICAL = 0b10000000;
   }
}

impl SpriteAttribute {
    pub fn palette(&self) -> u8 {
        self.bits & Self::PALETTE.bits
    }

    pub fn is_flip_horizontal(&self) -> bool {
        self.intersects(Self::FLIP_HORIZONTALLY)
    }

    pub fn is_flip_vertical(&self) -> bool {
        self.intersects(Self::FLIP_VERTICAL)
    }

    pub fn is_behind_background(&self) -> bool {
        self.intersects(Self::PRIORITY)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Sprite {
    x: u8,
    y: u8,
    tile_index: u8,
    attributes: SpriteAttribute,
    pattern: [u8; 2],
}
impl Sprite {
    pub fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            tile_index: 0,
            attributes: SpriteAttribute::empty(),
            pattern: [0; 2],
        }
    }

    // return a sprite which consists of 0xFF bytes only
    pub fn filled_ff() -> Self {
        Self {
            x: 0xFF,
            y: 0xFF,
            tile_index: 0xFF,
            attributes: SpriteAttribute::all(),
            pattern: [0xFF; 2],
        }
    }

    pub fn get_y(&self) -> u8 {
        self.y
    }

    /// for 8x8:
    /// use the byte normally as index into the pattern table
    ///
    /// for 8x16:
    /// 76543210
    /// ||||||||
    /// |||||||+- Bank ($0000 or $1000) of tiles
    /// +++++++-- Tile number of top of sprite (0 to 254; bottom half gets the next tile)
    pub fn get_tile(&self) -> u8 {
        self.tile_index
    }

    pub fn get_attribute(&self) -> SpriteAttribute {
        self.attributes
    }

    pub fn set_pattern(&mut self, pattern: [u8; 2]) {
        self.pattern = pattern;
    }

    pub fn get_color_bits(&self, x: u16) -> u8 {
        let mut x = x.wrapping_sub(self.x as u16);

        if x < 8 {
            if !self.attributes.is_flip_horizontal() {
                x = 7 - x;
            }

            (((self.pattern[1] >> x) & 1) << 1) | ((self.pattern[0] >> x) & 1)
        } else {
            0
        }
    }

    pub fn read_offset(&self, offset: u8) -> u8 {
        match offset {
            0 => {
                if self.y != 0 {
                    self.y - 1
                } else {
                    self.y
                }
            }
            1 => self.tile_index,
            2 => self.attributes.bits,
            3 => self.x,
            _ => unreachable!(),
        }
    }

    pub fn write_offset(&mut self, offset: u8, data: u8) {
        let to_change = match offset {
            0 => &mut self.y,
            1 => &mut self.tile_index,
            2 => &mut self.attributes.bits,
            3 => &mut self.x,
            _ => unreachable!(),
        };
        // y location is set to the position before the sprite (so weird)
        // but do not wrap 255
        *to_change = if offset == 0 && data != 255 {
            data + 1
        } else {
            data
        }
    }
}
