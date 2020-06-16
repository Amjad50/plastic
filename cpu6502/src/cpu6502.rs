use super::instruction::{AddressingMode, Instruction, Opcode};
use super::Bus;

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

pub struct CPU6502<'a> {
    reg_pc: u16,
    reg_sp: u8, // stack is in 0x0100 - 0x01FF only
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_status: u8,

    bus: &'a mut dyn Bus,
}

impl<'a> CPU6502<'a> {
    pub fn new(bus: &'a mut dyn Bus) -> Self {
        CPU6502 {
            reg_pc: 0,
            reg_sp: 0,
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_status: 0,

            bus: bus,
        }
    }

    fn set_flag(&mut self, flag: StatusFlag) {
        self.reg_status |= flag as u8;
    }

    fn unset_flag(&mut self, flag: StatusFlag) {
        self.reg_status &= !(flag as u8);
    }

    fn set_flag_status(&mut self, flag: StatusFlag, status: bool) {
        if status {
            self.set_flag(flag)
        } else {
            self.unset_flag(flag)
        }
    }

    fn decode_operand(&self, instruction: &Instruction) -> u16 {
        if instruction.is_operand_address() {
            match instruction.addressing_mode {
                AddressingMode::ZeroPage => instruction.operand & 0xff,
                AddressingMode::ZeroPageIndexX => (instruction.operand + self.reg_x as u16) & 0xff, // needs memory

                AddressingMode::ZeroPageIndexY => (instruction.operand + self.reg_y as u16) & 0xff, // needs memory

                AddressingMode::Indirect => {
                    let low = self.bus.read(instruction.operand) as u16;
                    let high = self.bus.read(instruction.operand + 1) as u16;
                    high << 8 | low
                }
                AddressingMode::XIndirect => {
                    let location_indirect = instruction.operand & 0xff + self.reg_x as u16;
                    let low = self.bus.read(location_indirect) as u16;
                    let high = self.bus.read(location_indirect + 1) as u16;
                    high << 8 | low
                }
                AddressingMode::IndirectY => {
                    let location_indirect = instruction.operand & 0xff;
                    let low = self.bus.read(location_indirect) as u16;
                    let high = self.bus.read(location_indirect + 1) as u16;

                    (high << 8 | low) + self.reg_y as u16
                }
                AddressingMode::Absolute => instruction.operand,
                AddressingMode::AbsoluteX => instruction.operand + self.reg_x as u16,
                AddressingMode::AbsoluteY => instruction.operand + self.reg_y as u16,
                AddressingMode::Relative => self.reg_pc + instruction.operand & 0xff,
                AddressingMode::Immediate
                | AddressingMode::Accumulator
                | AddressingMode::Implied => {
                    unreachable!();
                }
            }
        } else {
            instruction.operand
        }
    }

    pub fn run_instruction(&mut self, instruction: &Instruction) {
        let handler = match instruction.opcode {
            Opcode::Adc => Self::adc,
            Opcode::And => Self::and,
            Opcode::Asl => Self::asl,
            Opcode::Eor => Self::eor,
            Opcode::Lsr => Self::lsr,
            Opcode::Ora => Self::ora,
            Opcode::Rol => Self::rol,
            Opcode::Ror => Self::ror,
            Opcode::Sbc => Self::sbc,
            Opcode::Bit => Self::bit,
            Opcode::Cmp => Self::cmp,
            Opcode::Cpx => Self::cpx,
            Opcode::Cpy => Self::cpy,
            Opcode::Brk => Self::brk,
            Opcode::Bcc => Self::bcc,
            Opcode::Bcs => Self::bcs,
            Opcode::Beq => Self::beq,
            Opcode::Bmi => Self::bmi,
            Opcode::Bne => Self::bne,
            Opcode::Bpl => Self::bpl,
            Opcode::Bvc => Self::bvc,
            Opcode::Bvs => Self::bvs,
            Opcode::Dec => Self::dec,
            Opcode::Inc => Self::inc,
            Opcode::Clc => Self::clc,
            Opcode::Cld => Self::cld,
            Opcode::Cli => Self::cli,
            Opcode::Clv => Self::clv,
            Opcode::Sec => Self::sec,
            Opcode::Sed => Self::sed,
            Opcode::Sei => Self::sei,
            Opcode::Jmp => Self::jmp,
            Opcode::Jsr => Self::jsr,
            Opcode::Rti => Self::rti,
            Opcode::Rts => Self::rts,
            Opcode::Lda => Self::lda,
            Opcode::Ldx => Self::ldx,
            Opcode::Ldy => Self::ldy,
            Opcode::Nop => Self::nop,
            Opcode::Dex => Self::dex,
            Opcode::Dey => Self::dey,
            Opcode::Inx => Self::inx,
            Opcode::Iny => Self::iny,
            Opcode::Tax => Self::tax,
            Opcode::Tay => Self::tay,
            Opcode::Txa => Self::txa,
            Opcode::Tya => Self::tya,
            Opcode::Pha => Self::pha,
            Opcode::Php => Self::php,
            Opcode::Pla => Self::pla,
            Opcode::Plp => Self::plp,
            Opcode::Sta => Self::sta,
            Opcode::Stx => Self::stx,
            Opcode::Sty => Self::sty,
            Opcode::Tsx => Self::tsx,
            Opcode::Txs => Self::txs,
        };

        handler(
            self,
            self.decode_operand(instruction),
            instruction.is_operand_address(),
        );
    }

    // TODO: Add support for BCD mode, also handle cycles
    fn adc(&mut self, operand_decoded: u16, is_operand_address: bool) {
        let operand = if is_operand_address {
            self.bus.read(operand_decoded)
        } else {
            operand_decoded as u8
        };
        let carry = if self.reg_status & (StatusFlag::Carry as u8) == 0 {
            0
        } else {
            1
        };

        let result = self.reg_a as u16 + operand as u16 + carry;

        // overflow = result is negative ^ (reg_A is negative | operand is negative)
        // meaning, that if the operands are positive but the result is negative, then something
        // is not right, and the same way vise versa
        self.set_flag_status(
            StatusFlag::Overflow,
            (result as u8 & 0x80) ^ ((self.reg_a & 0x80) | (operand & 0x80)) != 0,
        );
        self.set_flag_status(StatusFlag::Carry, result & 0xff00 != 0);
        self.set_flag_status(StatusFlag::Zero, result == 0);
        self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

        self.reg_a = result as u8;
    }

    fn asl(&mut self, operand_decoded: u16, is_operand_address: bool) {
        let mut operand = if is_operand_address {
            self.bus.read(operand_decoded)
        } else {
            // if its not address, then its Accumulator for this instruction
            self.reg_a
        };

        // There is a bit at the leftmost position, it will be moved to the carry
        self.set_flag_status(StatusFlag::Carry, operand & 0x80 != 0);

        // modify the value
        operand <<= 1;

        self.set_flag_status(StatusFlag::Zero, operand == 0);
        self.set_flag_status(StatusFlag::Negative, operand & 0x80 != 0);

        if is_operand_address {
            // save back
            self.bus.write(operand_decoded, operand);
        } else {
            self.reg_a = operand;
        }
    }

    fn lsr(&mut self, operand_decoded: u16, is_operand_address: bool) {
        let mut operand = if is_operand_address {
            self.bus.read(operand_decoded)
        } else {
            // if its not address, then its Accumulator for this instruction
            self.reg_a
        };

        // There is a bit at the leftmost position, it will be moved to the carry
        self.set_flag_status(StatusFlag::Carry, operand & 0x01 != 0);

        // modify the value
        operand >>= 1;

        self.set_flag_status(StatusFlag::Zero, operand == 0);
        self.set_flag_status(StatusFlag::Negative, false);

        if is_operand_address {
            // save back
            self.bus.write(operand_decoded, operand);
        } else {
            self.reg_a = operand;
        }
    }

    fn rol(&mut self, operand_decoded: u16, is_operand_address: bool) {
        let mut operand = if is_operand_address {
            self.bus.read(operand_decoded)
        } else {
            // if its not address, then its Accumulator for this instruction
            self.reg_a
        };

        let old_carry = if self.reg_status & (StatusFlag::Carry as u8) == 0 {
            0
        } else {
            1
        };

        // There is a bit at the leftmost position, it will be moved to the carry
        self.set_flag_status(StatusFlag::Carry, operand & 0x01 != 0);

        // modify the value
        operand <<= 1;
        operand |= old_carry;

        self.set_flag_status(StatusFlag::Zero, operand == 0);
        self.set_flag_status(StatusFlag::Negative, false);

        if is_operand_address {
            // save back
            self.bus.write(operand_decoded, operand);
        } else {
            self.reg_a = operand;
        }
    }

    fn ror(&mut self, operand_decoded: u16, is_operand_address: bool) {
        let mut operand = if is_operand_address {
            self.bus.read(operand_decoded)
        } else {
            // if its not address, then its Accumulator for this instruction
            self.reg_a
        };

        let old_carry = if self.reg_status & (StatusFlag::Carry as u8) == 0 {
            0
        } else {
            1
        };

        // There is a bit at the leftmost position, it will be moved to the carry
        self.set_flag_status(StatusFlag::Carry, operand & 0x01 != 0);

        // modify the value
        operand >>= 1;
        operand |= old_carry << 7;

        self.set_flag_status(StatusFlag::Zero, operand == 0);
        self.set_flag_status(StatusFlag::Negative, false);

        if is_operand_address {
            // save back
            self.bus.write(operand_decoded, operand);
        } else {
            self.reg_a = operand;
        }
    }

    fn run_bitwise_operation<F>(&mut self, operand_decoded: u16, is_operand_address: bool, f: F)
    where
        F: Fn(u8, u8) -> u8,
    {
        let operand = if is_operand_address {
            self.bus.read(operand_decoded)
        } else {
            operand_decoded as u8
        };

        let result = f(operand, self.reg_a);

        self.set_flag_status(StatusFlag::Zero, result == 0);
        self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

        self.reg_a = result;
    }

    fn and(&mut self, operand_decoded: u16, is_operand_address: bool) {
        self.run_bitwise_operation(operand_decoded, is_operand_address, |a, b| a & b);
    }

    fn eor(&mut self, operand_decoded: u16, is_operand_address: bool) {
        self.run_bitwise_operation(operand_decoded, is_operand_address, |a, b| a ^ b);
    }

    fn ora(&mut self, operand_decoded: u16, is_operand_address: bool) {
        self.run_bitwise_operation(operand_decoded, is_operand_address, |a, b| a | b);
    }

    fn sbc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bit(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn cmp(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn cpx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn cpy(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn brk(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bcc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bcs(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn beq(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bmi(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bne(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bpl(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bvc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn bvs(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn dec(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn inc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn clc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn cld(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn cli(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn clv(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn sec(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn sed(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn sei(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn jmp(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn jsr(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn rti(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn rts(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn lda(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn ldx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn ldy(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn nop(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn dex(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn dey(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn inx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn iny(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn tax(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn tay(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn txa(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn tya(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn pha(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn php(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn pla(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn plp(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn sta(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn stx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn sty(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn tsx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn txs(&mut self, operand_decoded: u16, is_operand_address: bool) {}
}
