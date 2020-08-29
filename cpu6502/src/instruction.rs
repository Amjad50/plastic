use std::fmt::{Display, Formatter};

pub struct Instruction {
    pub opcode_byte: u8,
    pub operand: u16,
    pub opcode: Opcode,
    pub addressing_mode: AddressingMode,
}

#[derive(PartialEq, Eq, Copy, Clone)]
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

use Opcode::*;

#[rustfmt::skip]
const OPCODES: [Opcode;256] = [
//  0    1    2    3    4    5    6    7    8    9    A    B    C    D    E    F
    Brk, Ora, Kil, Slo, Nop, Ora, Asl, Slo, Php, Ora, Asl, Anc, Nop, Ora, Asl, Slo, // 00
    Bpl, Ora, Kil, Slo, Nop, Ora, Asl, Slo, Clc, Ora, Nop, Slo, Nop, Ora, Asl, Slo, // 10
    Jsr, And, Kil, Rla, Bit, And, Rol, Rla, Plp, And, Rol, Anc, Bit, And, Rol, Rla, // 20
    Bmi, And, Kil, Rla, Nop, And, Rol, Rla, Sec, And, Nop, Rla, Nop, And, Rol, Rla, // 30
    Rti, Eor, Kil, Sre, Nop, Eor, Lsr, Sre, Pha, Eor, Lsr, Alr, Jmp, Eor, Lsr, Sre, // 40
    Bvc, Eor, Kil, Sre, Nop, Eor, Lsr, Sre, Cli, Eor, Nop, Sre, Nop, Eor, Lsr, Sre, // 50
    Rts, Adc, Kil, Rra, Nop, Adc, Ror, Rra, Pla, Adc, Ror, Arr, Jmp, Adc, Ror, Rra, // 60
    Bvs, Adc, Kil, Rra, Nop, Adc, Ror, Rra, Sei, Adc, Nop, Rra, Nop, Adc, Ror, Rra, // 70
    Nop, Sta, Nop, Sax, Sty, Sta, Stx, Sax, Dey, Nop, Txa, Xaa, Sty, Sta, Stx, Sax, // 80
    Bcc, Sta, Kil, Ahx, Sty, Sta, Stx, Sax, Tya, Sta, Txs, Tas, Shy, Sta, Shx, Ahx, // 90
    Ldy, Lda, Ldx, Lax, Ldy, Lda, Ldx, Lax, Tay, Lda, Tax, Lax, Ldy, Lda, Ldx, Lax, // A0
    Bcs, Lda, Kil, Lax, Ldy, Lda, Ldx, Lax, Clv, Lda, Tsx, Las, Ldy, Lda, Ldx, Lax, // B0
    Cpy, Cmp, Nop, Dcp, Cpy, Cmp, Dec, Dcp, Iny, Cmp, Dex, Axs, Cpy, Cmp, Dec, Dcp, // C0
    Bne, Cmp, Kil, Dcp, Nop, Cmp, Dec, Dcp, Cld, Cmp, Nop, Dcp, Nop, Cmp, Dec, Dcp, // D0
    Cpx, Sbc, Nop, Isc, Cpx, Sbc, Inc, Isc, Inx, Sbc, Nop, Sbc, Cpx, Sbc, Inc, Isc, // E0
    Beq, Sbc, Kil, Isc, Nop, Sbc, Inc, Isc, Sed, Sbc, Nop, Isc, Nop, Sbc, Inc, Isc, // F0
];

use AddressingMode::*;

#[rustfmt::skip]
const ADDRESSING_MODES: [AddressingMode; 256] = [
//  0           1               2               3               4               5               6               7
    Implied,    XIndirect,      Implied,        XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // 00
    Implied,    Immediate,      Accumulator,    Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // 08
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, // 10
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteX,      AbsoluteX,      // 18
    Absolute,   XIndirect,      Implied,        XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // 20
    Implied,    Immediate,      Accumulator,    Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // 28
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, // 30
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteX,      AbsoluteX,      // 38
    Implied,    XIndirect,      Implied,        XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // 40
    Implied,    Immediate,      Accumulator,    Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // 48
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, // 50
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteX,      AbsoluteX,      // 58
    Implied,    XIndirect,      Implied,        XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // 60
    Implied,    Immediate,      Accumulator,    Immediate,      Indirect,       Absolute,       Absolute,       Absolute,       // 68
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, // 70
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteX,      AbsoluteX,      // 78
    Immediate,  XIndirect,      Immediate,      XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // 80
    Implied,    Immediate,      Implied,        Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // 88
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexY, ZeroPageIndexY, // 90
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteY,      AbsoluteY,      // 98
    Immediate,  XIndirect,      Immediate,      XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // A0
    Implied,    Immediate,      Implied,        Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // A8
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexY, ZeroPageIndexY, // B0
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteY,      AbsoluteY,      // B8
    Immediate,  XIndirect,      Immediate,      XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // C0
    Implied,    Immediate,      Implied,        Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // C8
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, // D0
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteX,      AbsoluteX,      // D8
    Immediate,  XIndirect,      Immediate,      XIndirect,      ZeroPage,       ZeroPage,       ZeroPage,       ZeroPage,       // E0
    Implied,    Immediate,      Implied,        Immediate,      Absolute,       Absolute,       Absolute,       Absolute,       // E8
    Relative,   IndirectY,      Implied,        IndirectY,      ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, ZeroPageIndexX, // F0
    Implied,    AbsoluteY,      Implied,        AbsoluteY,      AbsoluteX,      AbsoluteX,      AbsoluteX,      AbsoluteX,      // F8
];

// public
impl AddressingMode {
    pub fn can_cross_page(&self) -> bool {
        self == &AddressingMode::IndirectY
            || self == &AddressingMode::AbsoluteX
            || self == &AddressingMode::AbsoluteY
    }
}

// private
impl AddressingMode {
    fn get_instruction_len(&self) -> usize {
        match self {
            AddressingMode::Immediate => 2,
            AddressingMode::ZeroPage => 2,
            AddressingMode::ZeroPageIndexX => 2,
            AddressingMode::ZeroPageIndexY => 2,
            AddressingMode::Indirect => 3,
            AddressingMode::XIndirect => 2,
            AddressingMode::IndirectY => 2,
            AddressingMode::Absolute => 3,
            AddressingMode::AbsoluteX => 3,
            AddressingMode::AbsoluteY => 3,
            AddressingMode::Accumulator => 1,
            AddressingMode::Relative => 2,
            AddressingMode::Implied => 1,
        }
    }

    fn get_base_cycle_time(&self) -> u8 {
        match self {
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

    fn is_operand_address(&self) -> bool {
        // these do not have address as operand
        !(self == &AddressingMode::Accumulator
            || self == &AddressingMode::Implied
            || self == &AddressingMode::Immediate)
    }
}

impl Instruction {
    pub fn from_byte(byte: u8) -> Instruction {
        Instruction {
            opcode_byte: byte,
            operand: 0,
            opcode: OPCODES[byte as usize],
            addressing_mode: ADDRESSING_MODES[byte as usize],
        }
    }

    pub fn get_instruction_len(&self) -> usize {
        // the length of the instruction depend on the type of its addressing mode
        self.addressing_mode.get_instruction_len()
    }

    pub fn get_base_cycle_time(&self) -> u8 {
        // the base cycle time of the instruction depend on the type of its
        // addressing mode
        self.addressing_mode.get_base_cycle_time()
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
