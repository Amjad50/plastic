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

    // Unofficial instructions
    Slo, // Arithmetic shift left, then OR [Asl + Ora]
    Sre, // Logical shift right, then XOR [Lsr + Eor]
    Rla, // Rotate left, then And [Rol + And]
    Rra, // Rotate right, then Add with carry [Ror + Adc]
    Isc, // Increment memory, then Subtract with carry [Inc + Sbc]
    Dcp, // Decrement memory, then Compare A [Dec + Cmp]
    Sax, // Store the value of (A and X)
    Lax, // Load A and X [Lda + Tax]

    Anc, // And #imm, then set Carry = Negative
    Alr, // And #imm, then Lsr A
    Arr, // And #imm, then Ror A
    Axs, // ((A and X) - #imm) -> X
    Xaa, // Transfere X to A, then And #imm [Txa + And #imm]

    Ahx, // Store the value of (A and X and {High byte of $addr}) into $addr
    Shy, // Store the value of (Y and {High byte of $addr}) into $addr
    Shx, // Store the value of (X and {High byte of $addr}) into $addr

    Tas, // Store the value of (X and A) into (the stack pointer), and store ((the stack pointer) and {High byte of $addr}) into $addr
    Las, // Store the value of ({value in $addr} & (the stack pointer)) into A, X, and (the stack pointer)

    Kil, // Halt the CPU (CRASH)
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum AddressingMode {
    Immediate = 0,  // #$aa
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
        // mapped table for length of each type
        [2, 2, 2, 2, 3, 2, 2, 3, 3, 3, 1, 2, 1][*self as usize]
    }

    pub fn is_operand_address(&self) -> bool {
        // these do not have address as operand
        !(self == &AddressingMode::Accumulator
            || self == &AddressingMode::Implied
            || self == &AddressingMode::Immediate)
    }

    pub fn can_cross_page(&self) -> bool {
        self == &AddressingMode::IndirectY
            || self == &AddressingMode::AbsoluteX
            || self == &AddressingMode::AbsoluteY
    }
}

impl Instruction {
    // got this bit format from (http://nparker.llx.com/a2/opcodes.html)
    pub fn from_byte(byte: u8) -> Result<Instruction, ()> {
        let (opcode, addressing_mode) = match byte {
            0x05 => (Opcode::Ora, AddressingMode::ZeroPage),
            0x15 => (Opcode::Ora, AddressingMode::ZeroPageIndexX),
            0x0D => (Opcode::Ora, AddressingMode::Absolute),
            0x1D => (Opcode::Ora, AddressingMode::AbsoluteX),
            0x09 => (Opcode::Ora, AddressingMode::Immediate),
            0x19 => (Opcode::Ora, AddressingMode::AbsoluteY),
            0x01 => (Opcode::Ora, AddressingMode::XIndirect),
            0x11 => (Opcode::Ora, AddressingMode::IndirectY),

            0x25 => (Opcode::And, AddressingMode::ZeroPage),
            0x35 => (Opcode::And, AddressingMode::ZeroPageIndexX),
            0x2D => (Opcode::And, AddressingMode::Absolute),
            0x3D => (Opcode::And, AddressingMode::AbsoluteX),
            0x29 => (Opcode::And, AddressingMode::Immediate),
            0x39 => (Opcode::And, AddressingMode::AbsoluteY),
            0x21 => (Opcode::And, AddressingMode::XIndirect),
            0x31 => (Opcode::And, AddressingMode::IndirectY),

            0x45 => (Opcode::Eor, AddressingMode::ZeroPage),
            0x55 => (Opcode::Eor, AddressingMode::ZeroPageIndexX),
            0x4D => (Opcode::Eor, AddressingMode::Absolute),
            0x5D => (Opcode::Eor, AddressingMode::AbsoluteX),
            0x49 => (Opcode::Eor, AddressingMode::Immediate),
            0x59 => (Opcode::Eor, AddressingMode::AbsoluteY),
            0x41 => (Opcode::Eor, AddressingMode::XIndirect),
            0x51 => (Opcode::Eor, AddressingMode::IndirectY),

            0x65 => (Opcode::Adc, AddressingMode::ZeroPage),
            0x75 => (Opcode::Adc, AddressingMode::ZeroPageIndexX),
            0x6D => (Opcode::Adc, AddressingMode::Absolute),
            0x7D => (Opcode::Adc, AddressingMode::AbsoluteX),
            0x69 => (Opcode::Adc, AddressingMode::Immediate),
            0x79 => (Opcode::Adc, AddressingMode::AbsoluteY),
            0x61 => (Opcode::Adc, AddressingMode::XIndirect),
            0x71 => (Opcode::Adc, AddressingMode::IndirectY),

            0x85 => (Opcode::Sta, AddressingMode::ZeroPage),
            0x95 => (Opcode::Sta, AddressingMode::ZeroPageIndexX),
            0x8D => (Opcode::Sta, AddressingMode::Absolute),
            0x9D => (Opcode::Sta, AddressingMode::AbsoluteX),
            0x99 => (Opcode::Sta, AddressingMode::AbsoluteY),
            0x81 => (Opcode::Sta, AddressingMode::XIndirect),
            0x91 => (Opcode::Sta, AddressingMode::IndirectY),

            0xA5 => (Opcode::Lda, AddressingMode::ZeroPage),
            0xB5 => (Opcode::Lda, AddressingMode::ZeroPageIndexX),
            0xAD => (Opcode::Lda, AddressingMode::Absolute),
            0xBD => (Opcode::Lda, AddressingMode::AbsoluteX),
            0xA9 => (Opcode::Lda, AddressingMode::Immediate),
            0xB9 => (Opcode::Lda, AddressingMode::AbsoluteY),
            0xA1 => (Opcode::Lda, AddressingMode::XIndirect),
            0xB1 => (Opcode::Lda, AddressingMode::IndirectY),

            0xC5 => (Opcode::Cmp, AddressingMode::ZeroPage),
            0xD5 => (Opcode::Cmp, AddressingMode::ZeroPageIndexX),
            0xCD => (Opcode::Cmp, AddressingMode::Absolute),
            0xDD => (Opcode::Cmp, AddressingMode::AbsoluteX),
            0xC9 => (Opcode::Cmp, AddressingMode::Immediate),
            0xD9 => (Opcode::Cmp, AddressingMode::AbsoluteY),
            0xC1 => (Opcode::Cmp, AddressingMode::XIndirect),
            0xD1 => (Opcode::Cmp, AddressingMode::IndirectY),

            0xE5 => (Opcode::Sbc, AddressingMode::ZeroPage),
            0xF5 => (Opcode::Sbc, AddressingMode::ZeroPageIndexX),
            0xED => (Opcode::Sbc, AddressingMode::Absolute),
            0xFD => (Opcode::Sbc, AddressingMode::AbsoluteX),
            0xE9 => (Opcode::Sbc, AddressingMode::Immediate),
            0xF9 => (Opcode::Sbc, AddressingMode::AbsoluteY),
            0xE1 => (Opcode::Sbc, AddressingMode::XIndirect),
            0xF1 => (Opcode::Sbc, AddressingMode::IndirectY),

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

            // Unofficial instructions
            0x07 => (Opcode::Slo, AddressingMode::ZeroPage),
            0x17 => (Opcode::Slo, AddressingMode::ZeroPageIndexX),
            0x0F => (Opcode::Slo, AddressingMode::Absolute),
            0x1F => (Opcode::Slo, AddressingMode::AbsoluteX),
            0x1B => (Opcode::Slo, AddressingMode::AbsoluteY),
            0x03 => (Opcode::Slo, AddressingMode::XIndirect),
            0x13 => (Opcode::Slo, AddressingMode::IndirectY),

            0x47 => (Opcode::Sre, AddressingMode::ZeroPage),
            0x57 => (Opcode::Sre, AddressingMode::ZeroPageIndexX),
            0x4F => (Opcode::Sre, AddressingMode::Absolute),
            0x5F => (Opcode::Sre, AddressingMode::AbsoluteX),
            0x5B => (Opcode::Sre, AddressingMode::AbsoluteY),
            0x43 => (Opcode::Sre, AddressingMode::XIndirect),
            0x53 => (Opcode::Sre, AddressingMode::IndirectY),

            0x27 => (Opcode::Rla, AddressingMode::ZeroPage),
            0x37 => (Opcode::Rla, AddressingMode::ZeroPageIndexX),
            0x2F => (Opcode::Rla, AddressingMode::Absolute),
            0x3F => (Opcode::Rla, AddressingMode::AbsoluteX),
            0x3B => (Opcode::Rla, AddressingMode::AbsoluteY),
            0x23 => (Opcode::Rla, AddressingMode::XIndirect),
            0x33 => (Opcode::Rla, AddressingMode::IndirectY),

            0x67 => (Opcode::Rra, AddressingMode::ZeroPage),
            0x77 => (Opcode::Rra, AddressingMode::ZeroPageIndexX),
            0x6F => (Opcode::Rra, AddressingMode::Absolute),
            0x7F => (Opcode::Rra, AddressingMode::AbsoluteX),
            0x7B => (Opcode::Rra, AddressingMode::AbsoluteY),
            0x63 => (Opcode::Rra, AddressingMode::XIndirect),
            0x73 => (Opcode::Rra, AddressingMode::IndirectY),

            0xE7 => (Opcode::Isc, AddressingMode::ZeroPage),
            0xF7 => (Opcode::Isc, AddressingMode::ZeroPageIndexX),
            0xEF => (Opcode::Isc, AddressingMode::Absolute),
            0xFF => (Opcode::Isc, AddressingMode::AbsoluteX),
            0xFB => (Opcode::Isc, AddressingMode::AbsoluteY),
            0xE3 => (Opcode::Isc, AddressingMode::XIndirect),
            0xF3 => (Opcode::Isc, AddressingMode::IndirectY),

            0xC7 => (Opcode::Dcp, AddressingMode::ZeroPage),
            0xD7 => (Opcode::Dcp, AddressingMode::ZeroPageIndexX),
            0xCF => (Opcode::Dcp, AddressingMode::Absolute),
            0xDF => (Opcode::Dcp, AddressingMode::AbsoluteX),
            0xDB => (Opcode::Dcp, AddressingMode::AbsoluteY),
            0xC3 => (Opcode::Dcp, AddressingMode::XIndirect),
            0xD3 => (Opcode::Dcp, AddressingMode::IndirectY),

            0x87 => (Opcode::Sax, AddressingMode::ZeroPage),
            0x97 => (Opcode::Sax, AddressingMode::ZeroPageIndexY),
            0x8F => (Opcode::Sax, AddressingMode::Absolute),
            0x83 => (Opcode::Sax, AddressingMode::XIndirect),

            0xA7 => (Opcode::Lax, AddressingMode::ZeroPage),
            0xB7 => (Opcode::Lax, AddressingMode::ZeroPageIndexY),
            0xAF => (Opcode::Lax, AddressingMode::Absolute),
            0xAB => (Opcode::Lax, AddressingMode::Immediate),
            0xBF => (Opcode::Lax, AddressingMode::AbsoluteY),
            0xA3 => (Opcode::Lax, AddressingMode::XIndirect),
            0xB3 => (Opcode::Lax, AddressingMode::IndirectY),

            0x0B => (Opcode::Anc, AddressingMode::Immediate),
            0x2B => (Opcode::Anc, AddressingMode::Immediate),

            0x4B => (Opcode::Alr, AddressingMode::Immediate),

            0x6B => (Opcode::Arr, AddressingMode::Immediate),

            0xCB => (Opcode::Axs, AddressingMode::Immediate),

            0x8B => (Opcode::Xaa, AddressingMode::Immediate),

            0x93 => (Opcode::Ahx, AddressingMode::IndirectY),
            0x9F => (Opcode::Ahx, AddressingMode::AbsoluteY),

            0x9C => (Opcode::Shy, AddressingMode::AbsoluteX),

            0x9E => (Opcode::Shx, AddressingMode::AbsoluteY),

            0x9B => (Opcode::Tas, AddressingMode::AbsoluteY),

            0xBB => (Opcode::Las, AddressingMode::AbsoluteY),

            // duplicate
            0xEB => (Opcode::Sbc, AddressingMode::Immediate),

            // Nops
            0x04 | 0x44 | 0x64 => (Opcode::Nop, AddressingMode::ZeroPage),
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                (Opcode::Nop, AddressingMode::ZeroPageIndexX)
            }
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => (Opcode::Nop, AddressingMode::Implied),
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => (Opcode::Nop, AddressingMode::Immediate),
            0x0C => (Opcode::Nop, AddressingMode::Absolute),
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => (Opcode::Nop, AddressingMode::AbsoluteX),

            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                (Opcode::Kil, AddressingMode::Implied)
            }
        };

        Ok(Instruction {
            opcode_byte: byte,
            operand: 0,
            opcode,
            addressing_mode,
        })
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
            AddressingMode::IndirectY => 5, // might be 6 in case of page cross and STA
            AddressingMode::Absolute => 4,  // 3 for JMP, 6 for memory change and JSR
            AddressingMode::AbsoluteX => 4, // might be 5 in case of page cross and STA, and 7 in case of memory change
            AddressingMode::AbsoluteY => 4, // might be 5 in case of page cross and STA
            AddressingMode::Accumulator => 2,
            AddressingMode::Relative => 2,
            AddressingMode::Implied => 2, // should be overridden by instructions execution
        }
    }

    pub fn is_operand_address(&self) -> bool {
        self.addressing_mode.is_operand_address()
    }
}

#[cfg(not(tarpaulin_include))]
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

            // Unofficial instructions
            Slo => "SLO",
            Sre => "SRE",
            Rla => "RLA",
            Rra => "RRA",
            Isc => "ISC",
            Dcp => "DCP",
            Sax => "SAX",
            Lax => "LAX",

            Anc => "ANC",
            Alr => "ALR",
            Arr => "ARR",
            Axs => "AXS",
            Xaa => "XAA",

            Ahx => "AHX",
            Shy => "SHY",
            Shx => "SHX",

            Tas => "TAS",
            Las => "LAS",

            Kil => "KIL",
        };

        write!(f, "{}", result)
    }
}

#[cfg(not(tarpaulin_include))]
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
