use std::fmt::{Display, Formatter};

pub struct Instruction {
    pub opcode_byte: u8,
    pub operand: u16,
    pub opcode: Opcode,
    pub addressing_mode: AddressingMode,
}

#[derive(PartialEq, Eq)]
pub enum Opcode {
    Adc, // Add with carry
    And, // And
    Asl, // Arithmetic shift left
    Eor, // XOR
    Lsr, // Logical shift right
    Ora, // OR
    Rol, // Rotate left
    Ror, // Rotate right
    Sbc, // Subtract with carry

    Bit, // Test bits
    Cmp, // Compare A
    Cpx, // Compare X
    Cpy, // Compare Y

    Brk, // Break

    Bcc, // Branch on carry clear
    Bcs, // Branch on carry Set
    Beq, // Branch on equal
    Bmi, // Branch on minus
    Bne, // Branch on not equal
    Bpl, // Branch on plus
    Bvc, // Branch on overflow clear
    Bvs, // Branch on overflow set

    Dec, // Decrement memory
    Inc, // Increment memory

    Clc, // Clear carry
    Cld, // Clear decimal
    Cli, // Clear interrupt
    Clv, // Clear overflow
    Sec, // Set carry
    Sed, // Set decimal
    Sei, // Set interrupt

    Jmp, // Jump
    Jsr, // Jump to subroutine (call)
    Rti, // Return from interrupt
    Rts, // Return from subroutine

    Lda, // Load A
    Ldx, // Load X
    Ldy, // Load Y
    Nop, // No operation

    Dex, // Decrement X
    Dey, // Decrement Y
    Inx, // Increment X
    Iny, // Increment Y
    Tax, // Transfer A to X
    Tay, // Transfer A to Y
    Txa, // Transfer X to A
    Tya, // Transfer Y to A

    Pha, // Push A
    Php, // Push status register
    Pla, // Pull A
    Plp, // Pull status register
    Sta, // Store A
    Stx, // Store X
    Sty, // Store Y
    Tsx, // Transfer stack_ptr to X
    Txs, // Transfer X to stack_ptr
}

#[derive(PartialEq, Eq)]
pub enum AddressingMode {
    Immediate,      // #$aa
    ZeroPage,       // $aa
    ZeroPageIndexX, // $aa, X
    ZeroPageIndexY, // $aa, Y
    Indirect,       // ($aabb)
    XIndirect,      // ($aa, X)
    IndirectY,      // ($aa), Y
    Absolute,       // $aabb
    AbsoluteX,      // $aabb, X
    AbsoluteY,      // $aabb, Y
    Accumulator,    // A
    Relative,       // $aa (relative to current PC)
    Implied,        // Single byte instruction
}

impl AddressingMode {
    pub fn get_instruction_len(&self) -> usize {
        match self {
            Self::Immediate => 2,
            Self::ZeroPage => 2,
            Self::ZeroPageIndexX => 2,
            Self::ZeroPageIndexY => 2,
            Self::Indirect => 3,
            Self::XIndirect => 2,
            Self::IndirectY => 2,
            Self::Absolute => 3,
            Self::AbsoluteX => 3,
            Self::AbsoluteY => 3,
            Self::Accumulator => 1,
            Self::Relative => 2,
            Self::Implied => 1,
        }
    }

    pub fn is_operand_address(&self) -> bool {
        // these do not have address as operand
        !(self == &AddressingMode::Accumulator
            || self == &AddressingMode::Implied
            || self == &AddressingMode::Immediate)
    }
}

impl Instruction {
    // got this bit format from (http://nparker.llx.com/a2/opcodes.html)
    pub fn from_byte(byte: u8) -> Instruction {
        let cc = byte & 0b11;
        let addressing_mode_bbb = (byte >> 2) & 0b111;
        let opcode_aaa = (byte >> 5) & 0b111;

        let invalid_instruction_message = format!("Invalid instruction {:02x}", byte);

        let (opcode, addressing_mode) = match cc {
            0b01 => {
                let opcode = match opcode_aaa {
                    0b000 => Opcode::Ora,
                    0b001 => Opcode::And,
                    0b010 => Opcode::Eor,
                    0b011 => Opcode::Adc,
                    0b100 => Opcode::Sta,
                    0b101 => Opcode::Lda,
                    0b110 => Opcode::Cmp,
                    0b111 => Opcode::Sbc,
                    _ => unreachable!(),
                };
                let addressing_mode = match addressing_mode_bbb {
                    0b000 => AddressingMode::XIndirect,
                    0b001 => AddressingMode::ZeroPage,
                    0b010 => AddressingMode::Immediate,
                    0b011 => AddressingMode::Absolute,
                    0b100 => AddressingMode::IndirectY,
                    0b101 => AddressingMode::ZeroPageIndexX,
                    0b110 => AddressingMode::AbsoluteY,
                    0b111 => AddressingMode::AbsoluteX,
                    _ => unreachable!(),
                };

                // This instruction does not exists (STA with immediate)
                if opcode == Opcode::Sta && addressing_mode == AddressingMode::Immediate {
                    panic!(invalid_instruction_message)
                }

                (opcode, addressing_mode)
            }
            _ => match byte {
                0x0A => (Opcode::Asl, AddressingMode::Accumulator),
                0x06 => (Opcode::Asl, AddressingMode::ZeroPage),
                0x16 => (Opcode::Asl, AddressingMode::ZeroPageIndexX),
                0x0E => (Opcode::Asl, AddressingMode::Absolute),
                0x1E => (Opcode::Asl, AddressingMode::AbsoluteX),

                0x2A => (Opcode::Rol, AddressingMode::Accumulator),
                0x26 => (Opcode::Rol, AddressingMode::ZeroPage),
                0x36 => (Opcode::Rol, AddressingMode::ZeroPageIndexX),
                0x2E => (Opcode::Rol, AddressingMode::Absolute),
                0x3E => (Opcode::Rol, AddressingMode::AbsoluteX),

                0x4A => (Opcode::Lsr, AddressingMode::Accumulator),
                0x46 => (Opcode::Lsr, AddressingMode::ZeroPage),
                0x56 => (Opcode::Lsr, AddressingMode::ZeroPageIndexX),
                0x4E => (Opcode::Lsr, AddressingMode::Absolute),
                0x5E => (Opcode::Lsr, AddressingMode::AbsoluteX),

                0x6A => (Opcode::Ror, AddressingMode::Accumulator),
                0x66 => (Opcode::Ror, AddressingMode::ZeroPage),
                0x76 => (Opcode::Ror, AddressingMode::ZeroPageIndexX),
                0x6E => (Opcode::Ror, AddressingMode::Absolute),
                0x7E => (Opcode::Ror, AddressingMode::AbsoluteX),

                0x86 => (Opcode::Stx, AddressingMode::ZeroPage),
                0x96 => (Opcode::Stx, AddressingMode::ZeroPageIndexY),
                0x8E => (Opcode::Stx, AddressingMode::Absolute),

                0xA2 => (Opcode::Ldx, AddressingMode::Immediate),
                0xA6 => (Opcode::Ldx, AddressingMode::ZeroPage),
                0xB6 => (Opcode::Ldx, AddressingMode::ZeroPageIndexY),
                0xAE => (Opcode::Ldx, AddressingMode::Absolute),
                0xBE => (Opcode::Ldx, AddressingMode::AbsoluteY),

                0xC6 => (Opcode::Dec, AddressingMode::ZeroPage),
                0xD6 => (Opcode::Dec, AddressingMode::ZeroPageIndexX),
                0xCE => (Opcode::Dec, AddressingMode::Absolute),
                0xDE => (Opcode::Dec, AddressingMode::AbsoluteX),

                0xE6 => (Opcode::Inc, AddressingMode::ZeroPage),
                0xF6 => (Opcode::Inc, AddressingMode::ZeroPageIndexX),
                0xEE => (Opcode::Inc, AddressingMode::Absolute),
                0xFE => (Opcode::Inc, AddressingMode::AbsoluteX),

                0x24 => (Opcode::Bit, AddressingMode::ZeroPage),
                0x2C => (Opcode::Bit, AddressingMode::Absolute),

                0x4C => (Opcode::Jmp, AddressingMode::Absolute),
                0x6C => (Opcode::Jmp, AddressingMode::Indirect),

                0x84 => (Opcode::Sty, AddressingMode::ZeroPage),
                0x94 => (Opcode::Sty, AddressingMode::ZeroPageIndexX),
                0x8C => (Opcode::Sty, AddressingMode::Absolute),

                0xA0 => (Opcode::Ldy, AddressingMode::Immediate),
                0xA4 => (Opcode::Ldy, AddressingMode::ZeroPage),
                0xB4 => (Opcode::Ldy, AddressingMode::ZeroPageIndexX),
                0xAC => (Opcode::Ldy, AddressingMode::Absolute),
                0xBC => (Opcode::Ldy, AddressingMode::AbsoluteX),

                0xC0 => (Opcode::Cpy, AddressingMode::Immediate),
                0xC4 => (Opcode::Cpy, AddressingMode::ZeroPage),
                0xCC => (Opcode::Cpy, AddressingMode::Absolute),

                0xE0 => (Opcode::Cpx, AddressingMode::Immediate),
                0xE4 => (Opcode::Cpx, AddressingMode::ZeroPage),
                0xEC => (Opcode::Cpx, AddressingMode::Absolute),

                0x10 => (Opcode::Bpl, AddressingMode::Relative),
                0x30 => (Opcode::Bmi, AddressingMode::Relative),
                0x50 => (Opcode::Bvc, AddressingMode::Relative),
                0x70 => (Opcode::Bvs, AddressingMode::Relative),
                0x90 => (Opcode::Bcc, AddressingMode::Relative),
                0xB0 => (Opcode::Bcs, AddressingMode::Relative),
                0xD0 => (Opcode::Bne, AddressingMode::Relative),
                0xF0 => (Opcode::Beq, AddressingMode::Relative),

                0x00 => (Opcode::Brk, AddressingMode::Implied),
                0x20 => (Opcode::Jsr, AddressingMode::Absolute),
                0x40 => (Opcode::Rti, AddressingMode::Implied),
                0x60 => (Opcode::Rts, AddressingMode::Implied),

                0x08 => (Opcode::Php, AddressingMode::Implied),
                0x28 => (Opcode::Plp, AddressingMode::Implied),
                0x48 => (Opcode::Pha, AddressingMode::Implied),
                0x68 => (Opcode::Pla, AddressingMode::Implied),
                0x88 => (Opcode::Dey, AddressingMode::Implied),
                0xA8 => (Opcode::Tay, AddressingMode::Implied),
                0xC8 => (Opcode::Iny, AddressingMode::Implied),
                0xE8 => (Opcode::Inx, AddressingMode::Implied),

                0x18 => (Opcode::Clc, AddressingMode::Implied),
                0x38 => (Opcode::Sec, AddressingMode::Implied),
                0x58 => (Opcode::Cli, AddressingMode::Implied),
                0x78 => (Opcode::Sei, AddressingMode::Implied),
                0x98 => (Opcode::Tya, AddressingMode::Implied),
                0xB8 => (Opcode::Clv, AddressingMode::Implied),
                0xD8 => (Opcode::Cld, AddressingMode::Implied),
                0xF8 => (Opcode::Sed, AddressingMode::Implied),

                0x8A => (Opcode::Txa, AddressingMode::Implied),
                0x9A => (Opcode::Txs, AddressingMode::Implied),
                0xAA => (Opcode::Tax, AddressingMode::Implied),
                0xBA => (Opcode::Tsx, AddressingMode::Implied),
                0xCA => (Opcode::Dex, AddressingMode::Implied),
                0xEA => (Opcode::Nop, AddressingMode::Implied),

                _ => panic!(invalid_instruction_message),
            },
        };

        Instruction {
            opcode_byte: byte,
            operand: 0,
            opcode: opcode,
            addressing_mode: addressing_mode,
        }
    }

    pub fn get_instruction_len(&self) -> usize {
        // the length of the instruction depend on the type of its addressing mode
        self.addressing_mode.get_instruction_len()
    }

    pub fn get_base_cycle_time(&self) -> u8 {
        match self.addressing_mode {
            AddressingMode::Immediate => 2,
            AddressingMode::ZeroPage => 3, // or 5 for memory change
            AddressingMode::ZeroPageIndexX => 4, // or 6 for memory change
            AddressingMode::ZeroPageIndexY => 4,
            AddressingMode::Indirect => 5,
            AddressingMode::XIndirect => 6,
            AddressingMode::IndirectY => {
                if self.opcode == Opcode::Sta {
                    6
                } else {
                    5 // might be 6 in case of page cross
                }
            }
            AddressingMode::Absolute => 4, // 3 for JMP, 6 for memory change and JSR
            AddressingMode::AbsoluteX => {
                if self.opcode == Opcode::Sta {
                    5
                } else {
                    4 // might be 5 in case of page cross and 7 in case of memory change
                }
            }
            AddressingMode::AbsoluteY => {
                if self.opcode == Opcode::Sta {
                    5
                } else {
                    4 // might be 5 in case of page cross
                }
            }
            AddressingMode::Accumulator => 2,
            AddressingMode::Relative => 2,
            AddressingMode::Implied => 2, // should be overridden by instructions execution
        }
    }

    pub fn is_operand_address(&self) -> bool {
        self.addressing_mode.is_operand_address()
    }
}

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Opcode::*;
        let result = match *self {
            Adc => "ADC",
            And => "AND",
            Asl => "ASL",
            Eor => "EOR",
            Lsr => "LSR",
            Ora => "ORA",
            Rol => "ROL",
            Ror => "ROR",
            Sbc => "SBC",

            Bit => "BIT",
            Cmp => "CMP",
            Cpx => "CPX",
            Cpy => "CPY",

            Brk => "BRK",

            Bcc => "BCC",
            Bcs => "BCS",
            Beq => "BEQ",
            Bmi => "BMI",
            Bne => "BNE",
            Bpl => "BPL",
            Bvc => "BVC",
            Bvs => "BVS",

            Dec => "DEC",
            Inc => "INC",

            Clc => "CLC",
            Cld => "CLD",
            Cli => "CLI",
            Clv => "CLV",
            Sec => "SEC",
            Sed => "SED",
            Sei => "SEI",

            Jmp => "JMP",
            Jsr => "JSR",
            Rti => "RTI",
            Rts => "RTS",

            Lda => "LDA",
            Ldx => "LDX",
            Ldy => "LDY",
            Nop => "NOP",

            Dex => "DEX",
            Dey => "DEY",
            Inx => "INX",
            Iny => "INY",
            Tax => "TAX",
            Tay => "TAY",
            Txa => "TXA",
            Tya => "TYA",

            Pha => "PHA",
            Php => "PHP",
            Pla => "PLA",
            Plp => "PLP",
            Sta => "STA",
            Stx => "STX",
            Sty => "STY",
            Tsx => "TSX",
            Txs => "TXS",
        };

        write!(f, "{}", result)
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use AddressingMode::*;
        let addressing_string = match self.addressing_mode {
            Immediate => format!("#${:02X}", self.operand),
            ZeroPage => format!("${:02X}", self.operand),
            ZeroPageIndexX => format!("${:02X}, X", self.operand),
            ZeroPageIndexY => format!("${:02X}, Y", self.operand),
            Indirect => format!("(${:04X})", self.operand),
            XIndirect => format!("(${:02X}, X)", self.operand),
            IndirectY => format!("(${:02X}), Y", self.operand),
            Absolute => format!("${:04X}", self.operand),
            AbsoluteX => format!("${:04X}, X", self.operand),
            AbsoluteY => format!("${:04X}, Y", self.operand),
            Accumulator => format!("A"),
            Relative => format!("${:02X}", self.operand),
            Implied => format!(""),
        };

        write!(f, "{} {}", self.opcode, addressing_string)
    }
}
