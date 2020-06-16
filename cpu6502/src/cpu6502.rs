// flags: [N, V, _, B, D, I, Z, C]
enum StatusFlag {
    Carry = 1 << 0,
    Zero = 1 << 1,
    InterruptDisable = 1 << 2,
    DecimalMode = 1 << 3,
    BreakCommand = 1 << 4,
    Overflow = 1 << 6,
    Negative = 1 << 7,
}

pub struct CPU6502 {
    reg_pc: u16,
    reg_sp: u8,     // stack is in 0x0100 - 0x01FF only
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_status: u8,
}

impl CPU6502 {
    pub fn new() -> Self {
        CPU6502 {
            reg_pc: 0,
            reg_sp: 0,
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_status: 0,
        }
    }
}
