use crate::ppu2c02_registers::Register;
use common::Bus;

pub struct PPU2C02<'a, T: Bus> {
    // memory mapped registers
    reg_control: u8,
    reg_mask: u8,
    reg_status: u8,
    reg_oma_addr: u8,
    reg_oma_data: u8,
    reg_ppu_data: u8,
    reg_oma_dma: u8,

    scanline: u16,
    cycle: u16,

    vram_address_cur: u16,
    vram_address_tmp: u16,

    nametable_selector: u8, // 0, 1, 2, or 3 which maps to 0x2000, 0x2400, 0x2800, 0x2C00
    x_scroll: u8,
    y_scroll: u8,

    w_mode: bool,

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
            reg_status: 0,
            reg_oma_addr: 0,
            reg_oma_data: 0,
            reg_ppu_data: 0,
            reg_oma_dma: 0,

            scanline: 261, // start from -1 scanline
            cycle: 0,

            vram_address_cur: 0,
            vram_address_tmp: 0,

            nametable_selector: 0,
            x_scroll: 0,
            y_scroll: 0,

            w_mode: false,

            bus: bus,
        }
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            // reset w_mode
            Register::Status => self.reg_status,
            Register::DmaOma => self.reg_oma_dma,
            _ => {
                // unreadable
                0
            }
        }
    }

    pub(crate) fn write_register(&mut self, register: Register, data: u8) {
        match register {
            Register::Control => {
                self.reg_control = data;
                self.nametable_selector = data & 0b11;
            }
            Register::Mask => self.reg_mask = data,
            Register::OmaAddress => self.reg_oma_addr = data,
            Register::OmaData => self.reg_oma_data = data,
            Register::Scroll => {
                if self.w_mode {
                    self.x_scroll = data;
                } else {
                    self.y_scroll = data;
                }

                self.w_mode = !self.w_mode;
            }
            Register::PPUAddress => {
                if self.w_mode {
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
                    self.vram_address_cur = self.vram_address_tmp;
                }

                self.w_mode = !self.w_mode;
            }
            Register::PPUData => self.reg_ppu_data = data,
            Register::DmaOma => self.reg_oma_dma = data,
            _ => {
                // unwritable
            }
        };
    }

    // run one cycle, this should be fed from Master clock
    pub fn run_cycle(&mut self) {
        // current scanline
        match self.scanline {
            261 => {
                // pre-render
            }
            0..=239 => {
                // render
                self.run_render_cycle();
            }
            240 => {
                // post-render
            }
            241..=260 => {
                // vertical blanking
            }
            _ => {
                unreachable!();
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
                // main render
            }
            257..=320 => {}
            321..=340 => {}
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


    ## Attribute address ##
    NN 1111 YYY XXX
    || |||| ||| +++-- high 3 bits of coarse X (x/4)
    || |||| +++------ high 3 bits of coarse Y (y/4)
    || ++++---------- attribute offset (960 bytes)
    ++--------------- nametable select


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
}
