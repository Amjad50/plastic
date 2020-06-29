pub trait PPUCPUConnection {
    fn is_nmi_pin_set(&self) -> bool;
    fn clear_nmi_pin(&mut self);
}
