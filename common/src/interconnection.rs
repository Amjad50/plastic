pub trait PPUCPUConnection {
    fn is_nmi_pin_set(&self) -> bool;
    fn clear_nmi_pin(&mut self);
    fn is_dma_request(&self) -> bool;
    fn clear_dma_request(&mut self);
    fn dma_address(&mut self) -> u8;
    fn send_oam_data(&mut self, address: u8, data: u8);
}

pub trait CartridgeCPUConnection {
    fn is_irq_change_requested(&self) -> bool;
    fn irq_pin_state(&self) -> bool;
    fn clear_irq_request_pin(&mut self);
}
