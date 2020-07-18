#[derive(Debug, Copy, Clone)]
pub enum MirroringMode {
    Vertical,
    Horizontal,
    SingleScreenLowBank,
    SingleScreenHighBank,
    FourScreen,
}

pub trait MirroringProvider {
    fn mirroring_mode(&self) -> MirroringMode;
}
