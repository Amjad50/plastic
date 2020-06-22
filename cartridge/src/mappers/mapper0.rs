use crate::mapper::Mapper;

pub struct Mapper0 {
    has_32kb_prg_rom: bool,
}

impl Mapper0 {
    pub fn new() -> Self {
        Self {
            has_32kb_prg_rom: false,
        }
    }
}

impl Mapper for Mapper0 {
    fn init(&mut self, pgr_count: u8, _chr_count: u8) {
        self.has_32kb_prg_rom = pgr_count == 2;
    }

    fn map(&self, address: u16) -> u16 {
        assert!(address >= 0x8000 && address < 0xFFFF);

        if self.has_32kb_prg_rom {
            address
        } else {
            address & (0xBFFF)
        }
    }
}
