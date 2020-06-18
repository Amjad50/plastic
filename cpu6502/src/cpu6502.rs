use super::instruction::{AddressingMode, Instruction, Opcode};
use super::Bus;

// helper function
fn is_on_same_page(address1: u16, address2: u16) -> bool {
    address1 & 0xff00 == address2 & 0xff00
}

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
    pub reg_pc: u16, // FIXME: find better way to modify the PC for tests
    reg_sp: u8,      // stack is in 0x0100 - 0x01FF only
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
            reg_sp: 0xFD, // FIXME: not 100% about this
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
                    // if the indirect vector is at the last of the page (0xff) then
                    // wrap around on the same page
                    let high = self.bus.read(if instruction.operand & 0xff == 0xff {
                        instruction.operand & 0xff00
                    } else {
                        instruction.operand + 1
                    }) as u16;

                    (high << 8 | low, instruction.get_base_cycle_time())
                }
                AddressingMode::XIndirect => {
                    let location_indirect =
                        instruction.operand.wrapping_add(self.reg_x as u16) & 0xff;
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
                AddressingMode::Relative => {
                    let sign_extended_operand = instruction.operand
                        | if instruction.operand & 0x80 != 0 {
                            0xFF00
                        } else {
                            0x0000
                        };
                    (
                        self.reg_pc.wrapping_add(sign_extended_operand),
                        instruction.get_base_cycle_time(),
                    )
                }
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

    fn run_branch_condition(&mut self, decoded_operand: u16, condition: bool) -> u8 {
        let mut cycle_time = 0;
        if condition {
            cycle_time = if is_on_same_page(self.reg_pc, decoded_operand) {
                1
            } else {
                2
            };

            self.reg_pc = decoded_operand;
        }
        cycle_time
    }

    fn load(&mut self, decoded_operand: u16, is_operand_address: bool) -> u8 {
        let operand = if is_operand_address {
            self.bus.read(decoded_operand)
        } else {
            decoded_operand as u8
        };

        self.set_flag_status(StatusFlag::Zero, operand == 0);
        self.set_flag_status(StatusFlag::Negative, operand & 0x80 != 0);

        operand
    }

    fn push_stack(&mut self, data: u8) {
        self.bus.write(0x0100 | self.reg_sp as u16, data);
        self.reg_sp = self.reg_sp.wrapping_sub(1);
    }

    fn pull_stack(&mut self) -> u8 {
        self.reg_sp = self.reg_sp.wrapping_add(1);
        self.bus.read(0x0100 | self.reg_sp as u16)
    }

    pub fn run(&mut self) -> Result<(), u16>{
        // used to find infinite loops
        let mut last_pc = self.reg_pc;

        // loop until crash..
        loop {
            let instruction = self.fetch_next_instruction();

            // decode and execute
            self.run_instruction(&instruction);

            // if we stuck in a loop, return error
            if self.reg_pc == last_pc {
                return Err(self.reg_pc);
            } else {
                last_pc = self.reg_pc;
            }
        }
    }

    fn fetch_next_instruction(&mut self) -> Instruction {
        let opcode = self.bus.read(self.reg_pc);
        self.reg_pc += 1;

        let mut instruction = Instruction::from_byte(opcode);
        let mut operand = 0;
        // low
        if instruction.get_instruction_len() > 1 {
            operand |= self.bus.read(self.reg_pc) as u16;
            self.reg_pc += 1;
        }
        // high
        if instruction.get_instruction_len() > 2 {
            operand |= (self.bus.read(self.reg_pc) as u16) << 8;
            self.reg_pc += 1;
        }

        instruction.operand = operand;

        instruction
    }

    fn run_instruction(&mut self, instruction: &Instruction) {
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
                let result = (self.reg_a as u16)
                    .wrapping_add(operand as u16)
                    .wrapping_add(carry);
                // overflow = result is negative ^ (reg_A is negative | operand is negative)
                // meaning, that if the operands are positive but the result is negative, then something
                // is not right, and the same way vise versa
                self.set_flag_status(
                    StatusFlag::Overflow,
                    (((result as u8 ^ self.reg_a) & 0x80) != 0)
                        && !(((operand ^ self.reg_a) & 0x80) != 0),
                );
                self.set_flag_status(StatusFlag::Carry, result & 0xff00 != 0);
                self.set_flag_status(StatusFlag::Zero, result as u8 == 0);
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
                self.set_flag_status(StatusFlag::Carry, operand & 0x80 != 0);
                // modify the value
                operand <<= 1;
                operand |= old_carry;
                self.set_flag_status(StatusFlag::Zero, operand == 0);
                self.set_flag_status(StatusFlag::Negative, operand & 0x80 != 0);
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
                self.set_flag_status(StatusFlag::Negative, operand & 0x80 != 0);
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
                    ((result as u8 ^ self.reg_a) & 0x80 != 0)
                        && ((operand ^ self.reg_a) & 0x80 != 0),
                );
                self.set_flag_status(StatusFlag::Carry, !(result & 0xff00 != 0));
                self.set_flag_status(StatusFlag::Zero, result as u8 == 0);
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

                self.set_flag_status(StatusFlag::Zero, operand & self.reg_a == 0);
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
            Opcode::Bcc => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Carry as u8) == 0,
                );
            }
            Opcode::Bcs => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Carry as u8) != 0,
                );
            }
            Opcode::Beq => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Zero as u8) != 0,
                );
            }
            Opcode::Bmi => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Negative as u8) != 0,
                );
            }
            Opcode::Bne => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Zero as u8) == 0,
                );
            }
            Opcode::Bpl => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Negative as u8) == 0,
                );
            }
            Opcode::Bvc => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Overflow as u8) == 0,
                );
            }
            Opcode::Bvs => {
                cycle_time += self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Overflow as u8) != 0,
                );
            }
            Opcode::Dec => {
                assert!(is_operand_address);

                let result = self.bus.read(decoded_operand).wrapping_sub(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                // put back
                self.bus.write(decoded_operand, result);

                cycle_time += if instruction.addressing_mode == AddressingMode::AbsoluteX {
                    3
                } else {
                    2
                };
            }
            Opcode::Inc => {
                assert!(is_operand_address);

                let result = self.bus.read(decoded_operand).wrapping_add(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                // put back
                self.bus.write(decoded_operand, result);

                cycle_time += if instruction.addressing_mode == AddressingMode::AbsoluteX {
                    3
                } else {
                    2
                };
            }
            Opcode::Clc => {
                self.unset_flag(StatusFlag::Carry);
            }
            Opcode::Cld => {
                self.unset_flag(StatusFlag::DecimalMode);
            }
            Opcode::Cli => {
                self.unset_flag(StatusFlag::InterruptDisable);
            }
            Opcode::Clv => {
                self.unset_flag(StatusFlag::Overflow);
            }
            Opcode::Sec => {
                self.set_flag(StatusFlag::Carry);
            }
            Opcode::Sed => {
                self.set_flag(StatusFlag::DecimalMode);
            }
            Opcode::Sei => {
                self.set_flag(StatusFlag::InterruptDisable);
            }
            Opcode::Jmp => {
                assert!(is_operand_address);
                self.reg_pc = decoded_operand;

                cycle_time -= 1;
            }
            Opcode::Jsr => {
                assert!(is_operand_address);

                let pc = self.reg_pc - 1;
                let low = pc as u8;
                let high = (pc >> 8) as u8;

                self.push_stack(high);
                self.push_stack(low);

                self.reg_pc = decoded_operand;
            }
            Opcode::Rti => {
                self.reg_status = self.pull_stack();

                let low = self.pull_stack() as u16;
                let high = self.pull_stack() as u16;

                let address = high << 8 | low;

                // unlike RTS, this is the actual address
                self.reg_pc = address;
            }
            Opcode::Rts => {
                let low = self.pull_stack() as u16;
                let high = self.pull_stack() as u16;

                let address = high << 8 | low;

                // go to address + 1
                self.reg_pc = address + 1;
            }
            Opcode::Lda => {
                self.reg_a = self.load(decoded_operand, is_operand_address);
            }
            Opcode::Ldx => {
                self.reg_x = self.load(decoded_operand, is_operand_address);
            }
            Opcode::Ldy => {
                self.reg_y = self.load(decoded_operand, is_operand_address);
            }
            Opcode::Nop => {
                // NOTHING
            }
            Opcode::Dex => {
                let result = self.reg_x.wrapping_sub(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_x = result;
            }
            Opcode::Dey => {
                let result = self.reg_y.wrapping_sub(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_y = result;
            }
            Opcode::Inx => {
                let result = self.reg_x.wrapping_add(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_x = result;
            }
            Opcode::Iny => {
                let result = self.reg_y.wrapping_add(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_y = result;
            }
            Opcode::Tax => {
                let result = self.reg_a;

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_x = result;
            }
            Opcode::Tay => {
                let result = self.reg_a;

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_y = result;
            }
            Opcode::Txa => {
                let result = self.reg_x;

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_a = result;
            }
            Opcode::Tya => {
                let result = self.reg_y;

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_a = result;
            }
            Opcode::Pha => {
                self.push_stack(self.reg_a);
            }
            Opcode::Php => {
                self.push_stack(self.reg_status);
            }
            Opcode::Pla => {
                let result = self.pull_stack();

                // update flags
                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_a = result;
            }
            Opcode::Plp => {
                self.reg_status = self.pull_stack();
            }
            Opcode::Sta => {
                assert!(is_operand_address);
                self.bus.write(decoded_operand, self.reg_a);
            }
            Opcode::Stx => {
                assert!(is_operand_address);
                self.bus.write(decoded_operand, self.reg_x);
            }
            Opcode::Sty => {
                assert!(is_operand_address);
                self.bus.write(decoded_operand, self.reg_y);
            }
            Opcode::Tsx => {
                let result = self.reg_sp;

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_x = result;
            }
            Opcode::Txs => {
                // no need to set flags
                self.reg_sp = self.reg_x;
            }
        };

        // after finishing running the instruction
        // make sure the unused flag and B flag are always set
        // TODO: maybe there is a better way to do it?
        self.reg_status |= 0x30;

        self.cycles_to_wait = cycle_time;
    }
}
