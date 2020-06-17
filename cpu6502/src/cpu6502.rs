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

    cycles_to_wait: u8,

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

            cycles_to_wait: 0,

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

    fn decode_operand(&self, instruction: &Instruction) -> (u16, u8) {
        fn is_on_same_page(address1: u16, address2: u16) -> bool {
            address1 & 0xff00 == address2 & 0xff00
        }

        if instruction.is_operand_address() {
            match instruction.addressing_mode {
                AddressingMode::ZeroPage => (
                    instruction.operand & 0xff,
                    instruction.get_base_cycle_time(),
                ),
                AddressingMode::ZeroPageIndexX => (
                    (instruction.operand + self.reg_x as u16) & 0xff,
                    instruction.get_base_cycle_time(),
                ),

                AddressingMode::ZeroPageIndexY => (
                    (instruction.operand + self.reg_y as u16) & 0xff,
                    instruction.get_base_cycle_time(),
                ),

                AddressingMode::Indirect => {
                    let low = self.bus.read(instruction.operand) as u16;
                    let high = self.bus.read(instruction.operand + 1) as u16;
                    (high << 8 | low, instruction.get_base_cycle_time())
                }
                AddressingMode::XIndirect => {
                    let location_indirect = instruction.operand & 0xff + self.reg_x as u16;
                    let low = self.bus.read(location_indirect) as u16;
                    let high = self.bus.read(location_indirect + 1) as u16;
                    (high << 8 | low, instruction.get_base_cycle_time())
                }
                AddressingMode::IndirectY => {
                    let location_indirect = instruction.operand & 0xff;
                    let low = self.bus.read(location_indirect) as u16;
                    let high = self.bus.read(location_indirect + 1) as u16;

                    let unindxed_address = high << 8 | low;
                    let result = unindxed_address + self.reg_y as u16;

                    let page_cross = if is_on_same_page(unindxed_address, result) {
                        0
                    } else {
                        1
                    };

                    (result, instruction.get_base_cycle_time() + page_cross)
                }
                AddressingMode::Absolute => {
                    (instruction.operand, instruction.get_base_cycle_time())
                }
                AddressingMode::AbsoluteX => {
                    let result = instruction.operand + self.reg_x as u16;
                    let page_cross = if is_on_same_page(instruction.operand, result) {
                        0
                    } else {
                        1
                    };

                    (result, instruction.get_base_cycle_time() + page_cross)
                }
                AddressingMode::AbsoluteY => {
                    let result = instruction.operand + self.reg_y as u16;
                    let page_cross = if is_on_same_page(instruction.operand, result) {
                        0
                    } else {
                        1
                    };

                    (result, instruction.get_base_cycle_time() + page_cross)
                }
                AddressingMode::Relative => (
                    self.reg_pc + instruction.operand & 0xff,
                    instruction.get_base_cycle_time(),
                ),
                AddressingMode::Immediate
                | AddressingMode::Accumulator
                | AddressingMode::Implied => {
                    unreachable!();
                }
            }
        } else {
            (instruction.operand, instruction.get_base_cycle_time())
        }
    }

    fn run_bitwise_operation<F>(&mut self, decoded_operand: u16, is_operand_address: bool, f: F)
    where
        F: Fn(u8, u8) -> u8,
    {
        let operand = if is_operand_address {
            self.bus.read(decoded_operand)
        } else {
            decoded_operand as u8
        };

        let result = f(operand, self.reg_a);

        self.set_flag_status(StatusFlag::Zero, result == 0);
        self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

        self.reg_a = result;
    }

    fn run_cmp_operation(&mut self, decoded_operand: u16, is_operand_address: bool, register: u8) {
        let operand = if is_operand_address {
            self.bus.read(decoded_operand)
        } else {
            decoded_operand as u8
        };

        let result = (register as u16).wrapping_sub(operand as u16);

        self.set_flag_status(StatusFlag::Zero, result == 0);
        self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);
        self.set_flag_status(StatusFlag::Carry, result & 0xff00 == 0);
    }

    pub fn run_instruction(&mut self, instruction: &Instruction) {
        let (decoded_operand, cycle_time) = self.decode_operand(instruction);
        let mut cycle_time = cycle_time;

        let is_operand_address = instruction.is_operand_address();

        match instruction.opcode {
            // TODO: Add support for BCD mode
            Opcode::Adc => {
                let operand = if is_operand_address {
                    self.bus.read(decoded_operand)
                } else {
                    decoded_operand as u8
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

            Opcode::Asl => {
                let mut operand = if is_operand_address {
                    self.bus.read(decoded_operand)
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
                    self.bus.write(decoded_operand, operand);
                    cycle_time += 2;

                    // TODO: handle cycles better, PLEASE
                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::Lsr => {
                let mut operand = if is_operand_address {
                    self.bus.read(decoded_operand)
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
                    self.bus.write(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::Rol => {
                let mut operand = if is_operand_address {
                    self.bus.read(decoded_operand)
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
                    self.bus.write(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::Ror => {
                let mut operand = if is_operand_address {
                    self.bus.read(decoded_operand)
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
                    self.bus.write(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::And => {
                self.run_bitwise_operation(decoded_operand, is_operand_address, |a, b| a & b);
            }
            Opcode::Eor => {
                self.run_bitwise_operation(decoded_operand, is_operand_address, |a, b| a ^ b);
            }
            Opcode::Ora => {
                self.run_bitwise_operation(decoded_operand, is_operand_address, |a, b| a | b);
            }
            // TODO: Add support for BCD mode
            Opcode::Sbc => {
                let operand = if is_operand_address {
                    self.bus.read(decoded_operand)
                } else {
                    decoded_operand as u8
                };
                // inverse the carry
                let carry = if !(self.reg_status & (StatusFlag::Carry as u8) == 0) {
                    0
                } else {
                    1
                };
                let result = (self.reg_a as u16)
                    .wrapping_sub(operand as u16)
                    .wrapping_sub(carry);
                // overflow = (result's sign) & (2nd operand's sign) & !(1st operand's sign)
                // this was obtained from binary table
                self.set_flag_status(
                    StatusFlag::Overflow,
                    (result as u8 & 0x80) & (operand & 0x80) & !(self.reg_a & 0x80) != 0,
                );
                self.set_flag_status(StatusFlag::Carry, result & 0xff00 == 0);
                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);
                self.reg_a = result as u8;
            }
            Opcode::Bit => {
                // only Absolute and Zero page
                assert!(is_operand_address);
                let operand = self.bus.read(decoded_operand);
                // move the negative and overflow flags to the status register
                self.set_flag_status(
                    StatusFlag::Negative,
                    operand & StatusFlag::Negative as u8 != 0,
                );
                self.set_flag_status(
                    StatusFlag::Overflow,
                    operand & StatusFlag::Overflow as u8 != 0,
                );

                self.set_flag_status(StatusFlag::Zero, operand & self.reg_a != 0);
            }
            Opcode::Cmp => {
                self.run_cmp_operation(decoded_operand, is_operand_address, self.reg_a);
            }
            Opcode::Cpx => {
                self.run_cmp_operation(decoded_operand, is_operand_address, self.reg_x);
            }
            Opcode::Cpy => {
                self.run_cmp_operation(decoded_operand, is_operand_address, self.reg_y);
            }
            Opcode::Brk => {
                // TODO: implement later, don't know what is this
            }
            Opcode::Bcc => {}
            Opcode::Bcs => {}
            Opcode::Beq => {}
            Opcode::Bmi => {}
            Opcode::Bne => {}
            Opcode::Bpl => {}
            Opcode::Bvc => {}
            Opcode::Bvs => {}
            Opcode::Dec => {}
            Opcode::Inc => {}
            Opcode::Clc => {}
            Opcode::Cld => {}
            Opcode::Cli => {}
            Opcode::Clv => {}
            Opcode::Sec => {}
            Opcode::Sed => {}
            Opcode::Sei => {}
            Opcode::Jmp => {}
            Opcode::Jsr => {}
            Opcode::Rti => {}
            Opcode::Rts => {}
            Opcode::Lda => {}
            Opcode::Ldx => {}
            Opcode::Ldy => {}
            Opcode::Nop => {}
            Opcode::Dex => {}
            Opcode::Dey => {}
            Opcode::Inx => {}
            Opcode::Iny => {}
            Opcode::Tax => {}
            Opcode::Tay => {}
            Opcode::Txa => {}
            Opcode::Tya => {}
            Opcode::Pha => {}
            Opcode::Php => {}
            Opcode::Pla => {}
            Opcode::Plp => {}
            Opcode::Sta => {}
            Opcode::Stx => {}
            Opcode::Sty => {}
            Opcode::Tsx => {}
            Opcode::Txs => {}
        };

        self.cycles_to_wait = cycle_time;
    }
}
