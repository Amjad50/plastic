pub trait Mapper {
    fn init(&mut self, pgr_count: u8, chr_count: u8);
    fn map(&self, address: u16) -> u16;
}
