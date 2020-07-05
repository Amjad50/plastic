bitflags! {
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

#[derive(Copy, Clone)]
pub struct Sprite {
    x: u8,
    y: u8,
    tile_index: u8,
    attributes: SpriteAttribute,
}
impl Sprite {
    pub fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            tile_index: 0,
            attributes: SpriteAttribute::empty(),
        }
    }

    // return a sprite which consists of 0xFF bytes only
    pub fn filled_ff() -> Self {
        Self {
            x: 0xFF,
            y: 0xFF,
            tile_index: 0xFF,
            attributes: SpriteAttribute::all(),
        }
    }

    pub fn get_y(&self) -> u8 {
        self.y
    }

    pub fn get_x(&self) -> u8 {
        self.x
    }

    /*
    for 8x8:
    use the byte normally as index into the pattern table

    for 8x16:

    76543210
    ||||||||
    |||||||+- Bank ($0000 or $1000) of tiles
    +++++++-- Tile number of top of sprite (0 to 254; bottom half gets the next tile)
     */

    pub fn get_tile(&self) -> u8 {
        // FIXME: add support for 8x16
        self.tile_index
    }

    pub fn get_attribute(&self) -> SpriteAttribute {
        self.attributes
    }

    pub fn read_offset(&self, offset: u8) -> u8 {
        match offset {
            0 => self.y,
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
        *to_change = data;
    }
}
