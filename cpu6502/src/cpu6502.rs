use super::instruction::{AddressingMode, Instruction, Opcode};
use common::{
    interconnection::{APUCPUConnection, CpuIrqProvider, PPUCPUConnection},
    Bus, Device,
};
use std::{cell::RefCell, rc::Rc};

const NMI_VECTOR_ADDRESS: u16 = 0xFFFA;
const RESET_VECTOR_ADDRESS: u16 = 0xFFFC;
const IRQ_VECTOR_ADDRESS: u16 = 0xFFFE;

#[derive(PartialEq)]
pub enum CPURunState {
    DmaTransfere,
    Waiting,
    InfiniteLoop(u16),
    StartingInterrupt,
    NormalInstructionExecution,
}

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

// TODO: this CPU does not support BCD mode yet
pub struct CPU6502<T: Bus> {
    reg_pc: u16,
    reg_sp: u8,
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_status: u8,

    nmi_pin_status: bool,
    irq_pin_status: bool,

    cycles_to_wait: u8,

    dma_remaining: u16,
    dma_address: u8,

    bus: Rc<RefCell<T>>,
    ppu: Rc<RefCell<dyn PPUCPUConnection>>,
    apu: Rc<RefCell<dyn APUCPUConnection>>,

    irq_providers: Vec<Rc<RefCell<dyn CpuIrqProvider>>>,
}

impl<T> CPU6502<T>
where
    T: Bus,
{
    pub fn new(
        bus: Rc<RefCell<T>>,
        ppu: Rc<RefCell<dyn PPUCPUConnection>>,
        apu: Rc<RefCell<dyn APUCPUConnection>>,
    ) -> Self {
        CPU6502 {
            reg_pc: 0,
            reg_sp: 0xFD, // FIXME: not 100% about this
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_status: 0,

            nmi_pin_status: false,
            irq_pin_status: false,

            cycles_to_wait: 0,

            dma_remaining: 0,
            dma_address: 0,

            bus,
            ppu,
            apu,

            irq_providers: Vec::new(),
        }
    }

    pub fn add_irq_provider(&mut self, provider: Rc<RefCell<dyn CpuIrqProvider>>) {
        self.irq_providers.push(provider);
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

    fn read_bus(&self, address: u16) -> u8 {
        self.bus.borrow().read(address, Device::CPU)
    }

    fn write_bus(&mut self, address: u16, data: u8) {
        self.bus.borrow_mut().write(address, data, Device::CPU);
    }

    fn decode_operand(&self, instruction: &Instruction) -> (u16, u8, bool) {
        if instruction.is_operand_address() {
            match instruction.addressing_mode {
                AddressingMode::ZeroPage => (
                    instruction.operand & 0xff,
                    instruction.get_base_cycle_time(),
                    false,
                ),
                AddressingMode::ZeroPageIndexX => (
                    (instruction.operand + self.reg_x as u16) & 0xff,
                    instruction.get_base_cycle_time(),
                    false,
                ),

                AddressingMode::ZeroPageIndexY => (
                    (instruction.operand + self.reg_y as u16) & 0xff,
                    instruction.get_base_cycle_time(),
                    false,
                ),

                AddressingMode::Indirect => {
                    let low = self.read_bus(instruction.operand) as u16;
                    // if the indirect vector is at the last of the page (0xff) then
                    // wrap around on the same page
                    let high = self.read_bus(if instruction.operand & 0xff == 0xff {
                        instruction.operand & 0xff00
                    } else {
                        instruction.operand + 1
                    }) as u16;

                    (high << 8 | low, instruction.get_base_cycle_time(), false)
                }
                AddressingMode::XIndirect => {
                    let location_indirect =
                        instruction.operand.wrapping_add(self.reg_x as u16) & 0xff;
                    let low = self.read_bus(location_indirect) as u16;
                    let high = self.read_bus((location_indirect + 1) & 0xFF) as u16;
                    (high << 8 | low, instruction.get_base_cycle_time(), false)
                }
                AddressingMode::IndirectY => {
                    let location_indirect = instruction.operand & 0xff;
                    let low = self.read_bus(location_indirect) as u16;
                    let high = self.read_bus((location_indirect + 1) & 0xFF) as u16;

                    let unindxed_address = high << 8 | low;
                    let result = unindxed_address + self.reg_y as u16;

                    let page_cross = if is_on_same_page(unindxed_address, result) {
                        0
                    } else {
                        1
                    };

                    (
                        result,
                        instruction.get_base_cycle_time() + page_cross,
                        page_cross == 1,
                    )
                }
                AddressingMode::Absolute => (
                    instruction.operand,
                    instruction.get_base_cycle_time(),
                    false,
                ),
                AddressingMode::AbsoluteX => {
                    let result = instruction.operand + self.reg_x as u16;
                    let page_cross = if is_on_same_page(instruction.operand, result) {
                        0
                    } else {
                        1
                    };

                    (
                        result,
                        instruction.get_base_cycle_time() + page_cross,
                        page_cross == 1,
                    )
                }
                AddressingMode::AbsoluteY => {
                    let result = instruction.operand + self.reg_y as u16;
                    let page_cross = if is_on_same_page(instruction.operand, result) {
                        0
                    } else {
                        1
                    };

                    (
                        result,
                        instruction.get_base_cycle_time() + page_cross,
                        page_cross == 1,
                    )
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
                        false,
                    )
                }
                AddressingMode::Immediate
                | AddressingMode::Accumulator
                | AddressingMode::Implied => {
                    unreachable!();
                }
            }
        } else {
            (
                instruction.operand,
                instruction.get_base_cycle_time(),
                false,
            )
        }
    }

    fn run_bitwise_operation<F>(&mut self, decoded_operand: u16, is_operand_address: bool, f: F)
    where
        F: Fn(u8, u8) -> u8,
    {
        let operand = if is_operand_address {
            self.read_bus(decoded_operand)
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
            self.read_bus(decoded_operand)
        } else {
            decoded_operand as u8
        };

        let result = (register as u16).wrapping_sub(operand as u16);

        self.set_flag_status(StatusFlag::Zero, result == 0);
        self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);
        self.set_flag_status(StatusFlag::Carry, result & 0xff00 == 0);
    }

    fn run_branch_condition(&mut self, decoded_operand: u16, condition: bool) -> (u8, CPURunState) {
        let mut cycle_time = 0;

        // all branch instructions are 2 bytes, its hardcoded number
        // not sure if its good or not
        let pc = self.reg_pc.wrapping_sub(2);

        if condition {
            cycle_time = if is_on_same_page(self.reg_pc, decoded_operand) {
                1
            } else {
                2
            };

            self.reg_pc = decoded_operand;
        }

        (
            cycle_time,
            if condition && decoded_operand == pc {
                CPURunState::InfiniteLoop(pc)
            } else {
                CPURunState::NormalInstructionExecution
            },
        )
    }

    fn load(&mut self, decoded_operand: u16, is_operand_address: bool) -> u8 {
        let operand = if is_operand_address {
            self.read_bus(decoded_operand)
        } else {
            decoded_operand as u8
        };

        self.set_flag_status(StatusFlag::Zero, operand == 0);
        self.set_flag_status(StatusFlag::Negative, operand & 0x80 != 0);

        operand
    }

    fn push_stack(&mut self, data: u8) {
        self.write_bus(0x0100 | self.reg_sp as u16, data);
        self.reg_sp = self.reg_sp.wrapping_sub(1);
    }

    fn pull_stack(&mut self) -> u8 {
        self.reg_sp = self.reg_sp.wrapping_add(1);
        self.read_bus(0x0100 | self.reg_sp as u16)
    }

    pub fn reset(&mut self) {
        self.set_flag(StatusFlag::InterruptDisable);
        self.reg_sp = 0xFD; //reset

        let low = self.read_bus(RESET_VECTOR_ADDRESS) as u16;
        let high = self.read_bus(RESET_VECTOR_ADDRESS + 1) as u16;

        let pc = high << 8 | low;
        self.reg_pc = pc;
    }

    // is_soft should be only from BRK
    fn execute_interrupt(&mut self, is_soft: bool, is_nmi: bool) {
        let pc = self.reg_pc;

        let low = pc as u8;
        let high = (pc >> 8) as u8;

        self.push_stack(high);
        self.push_stack(low);

        self.set_flag_status(StatusFlag::BreakCommand, is_soft);

        self.push_stack(self.reg_status);

        let jump_vector_address = if is_nmi {
            NMI_VECTOR_ADDRESS
        } else {
            IRQ_VECTOR_ADDRESS
        };

        if is_nmi {
            // disable after execution, not to stuck in a infinite loop here
            self.nmi_pin_status = false;
        } else {
            self.irq_pin_status = false;
        }

        self.set_flag(StatusFlag::InterruptDisable);

        let low = self.read_bus(jump_vector_address) as u16;
        let high = self.read_bus(jump_vector_address + 1) as u16;

        let pc = high << 8 | low;
        self.reg_pc = pc;

        // delay of interrupt
        self.cycles_to_wait += 7;
    }

    fn check_for_nmi_dma(&mut self) {
        let mut ppu = self.ppu.borrow_mut();
        // check if the PPU is setting the NMI pin
        if ppu.is_nmi_pin_set() {
            self.nmi_pin_status = true;
            ppu.clear_nmi_pin();
        }
        // check if PPU is requesting DMA
        if ppu.is_dma_request() {
            self.dma_address = ppu.dma_address();
            self.dma_remaining = 256;
            ppu.clear_dma_request();
        }
    }

    fn check_for_irq(&mut self) {
        let mut is_irq_set = false;

        for provider in self.irq_providers.iter() {
            let mut provider = provider.borrow_mut();

            if provider.is_irq_change_requested() {
                if provider.irq_pin_state() {
                    is_irq_set = true;
                }
                self.irq_pin_status = provider.irq_pin_state();
                provider.clear_irq_request_pin();
            }
        }

        if is_irq_set {
            self.irq_pin_status = true;
        }
    }

    fn check_and_run_dmc_transfer(&mut self) {
        let request = self.apu.borrow().request_dmc_reader_read();

        if let Some(addr) = request {
            let data = self.read_bus(addr);

            self.apu.borrow_mut().submit_buffer_byte(data);

            // FIXME: respect different clock delay for respective positions to
            //  steal the clock
            self.cycles_to_wait += 3;
        }
    }

    // return true if an instruction executed
    // false if it was waiting for remaining cycles
    pub fn run_next(&mut self) -> CPURunState {
        self.check_and_run_dmc_transfer();

        if self.cycles_to_wait == 0 {
            // are we still executing the DMA transfer instruction?
            if self.dma_remaining > 0 {
                self.dma_remaining -= 1;
                {
                    // send one byte at a time
                    let oma_address = (255 - self.dma_remaining) & 0xFF;
                    let cpu_address = (self.dma_address as u16) << 8 | oma_address;

                    let data = self.read_bus(cpu_address);

                    self.ppu.borrow_mut().send_oam_data(oma_address as u8, data);
                }

                // since it should read in one cycle and write in the other cycle
                self.cycles_to_wait = 1;
                CPURunState::DmaTransfere
            } else {
                // interrupts waiting
                if self.nmi_pin_status
                    || (self.irq_pin_status
                        && !(self.reg_status & (StatusFlag::InterruptDisable as u8) != 0))
                {
                    // hardware side interrupt
                    self.execute_interrupt(false, self.nmi_pin_status);
                    CPURunState::StartingInterrupt
                } else {
                    // check for NMI and DMA and apply them only after the next
                    // instruction
                    self.check_for_nmi_dma();
                    // check if there is pending IRQs from cartridge
                    self.check_for_irq();

                    // fetch
                    let instruction = self.fetch_next_instruction();

                    // decode and execute
                    self.run_instruction(&instruction)
                }
            }
        } else {
            self.cycles_to_wait -= 1;
            CPURunState::Waiting
        }
    }

    fn fetch_next_instruction(&mut self) -> Instruction {
        let opcode = self.read_bus(self.reg_pc);
        self.reg_pc += 1;

        let mut instruction = match Instruction::from_byte(opcode) {
            Ok(instruction) => instruction,
            Err(_) => panic!(
                "Invalid instruction {:02X} at {:04X}",
                opcode,
                self.reg_pc - 1
            ),
        };

        let mut operand = 0;
        // low
        if instruction.get_instruction_len() > 1 {
            operand |= self.read_bus(self.reg_pc) as u16;
            self.reg_pc += 1;
        }
        // high
        if instruction.get_instruction_len() > 2 {
            operand |= (self.read_bus(self.reg_pc) as u16) << 8;
            self.reg_pc += 1;
        }

        instruction.operand = operand;

        instruction
    }

    fn run_instruction(&mut self, instruction: &Instruction) -> CPURunState {
        let (decoded_operand, cycle_time, did_page_cross) = self.decode_operand(instruction);
        let mut cycle_time = cycle_time;

        let is_operand_address = instruction.is_operand_address();

        let mut state = CPURunState::NormalInstructionExecution;

        match instruction.opcode {
            // TODO: Add support for BCD mode
            Opcode::Adc => {
                let operand = if is_operand_address {
                    self.read_bus(decoded_operand)
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
                    self.read_bus(decoded_operand)
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
                    self.write_bus(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    } else {
                        cycle_time += 2;
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::Lsr => {
                let mut operand = if is_operand_address {
                    self.read_bus(decoded_operand)
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
                    self.write_bus(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    } else {
                        cycle_time += 2;
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::Rol => {
                let mut operand = if is_operand_address {
                    self.read_bus(decoded_operand)
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
                    self.write_bus(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    } else {
                        cycle_time += 2;
                    }
                } else {
                    self.reg_a = operand;
                }
            }
            Opcode::Ror => {
                let mut operand = if is_operand_address {
                    self.read_bus(decoded_operand)
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
                    self.write_bus(decoded_operand, operand);

                    if instruction.addressing_mode == AddressingMode::AbsoluteX {
                        cycle_time = 7; // special case
                    } else {
                        cycle_time += 2;
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
                    self.read_bus(decoded_operand)
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
                let operand = self.read_bus(decoded_operand);
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
                // increment the PC for saving
                self.reg_pc += 1;
                self.execute_interrupt(true, self.nmi_pin_status);
                // execute_interrupt will add 7 and this instruction is implied so 2
                // but this instruction only takes 7 not 9, so minus 2
                self.cycles_to_wait -= 2;
            }
            Opcode::Bcc => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Carry as u8) == 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Bcs => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Carry as u8) != 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Beq => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Zero as u8) != 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Bmi => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Negative as u8) != 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Bne => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Zero as u8) == 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Bpl => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Negative as u8) == 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Bvc => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Overflow as u8) == 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Bvs => {
                let (time, run_state) = self.run_branch_condition(
                    decoded_operand,
                    self.reg_status & (StatusFlag::Overflow as u8) != 0,
                );
                cycle_time += time;
                state = run_state;
            }
            Opcode::Dec => {
                assert!(is_operand_address);

                let result = self.read_bus(decoded_operand).wrapping_sub(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                // put back
                self.write_bus(decoded_operand, result);

                if instruction.addressing_mode == AddressingMode::AbsoluteX {
                    cycle_time = 7; // special case
                } else {
                    cycle_time += 2;
                };
            }
            Opcode::Inc => {
                assert!(is_operand_address);

                let result = self.read_bus(decoded_operand).wrapping_add(1);

                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                // put back
                self.write_bus(decoded_operand, result);

                if instruction.addressing_mode == AddressingMode::AbsoluteX {
                    cycle_time = 7; // special case
                } else {
                    cycle_time += 2;
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

                // this instruction is 3 bytes long in both addressing variants
                let pc = self.reg_pc.wrapping_sub(3);

                self.reg_pc = decoded_operand;

                if pc == decoded_operand {
                    state = CPURunState::InfiniteLoop(pc);
                }

                // this instruction has only `Absolute` and `Relative` as adressing modes
                cycle_time = if instruction.addressing_mode == AddressingMode::Absolute {
                    3
                } else {
                    5
                };
            }
            Opcode::Jsr => {
                assert!(is_operand_address);

                let pc = self.reg_pc - 1;
                let low = pc as u8;
                let high = (pc >> 8) as u8;

                self.push_stack(high);
                self.push_stack(low);

                self.reg_pc = decoded_operand;

                cycle_time = 6;
            }
            Opcode::Rti => {
                let old_status = self.reg_status & 0x30;
                self.reg_status = self.pull_stack() | old_status;

                let low = self.pull_stack() as u16;
                let high = self.pull_stack() as u16;

                let address = high << 8 | low;

                // unlike RTS, this is the actual address
                self.reg_pc = address;

                cycle_time = 6;
            }
            Opcode::Rts => {
                let low = self.pull_stack() as u16;
                let high = self.pull_stack() as u16;

                let address = high << 8 | low;

                // go to address + 1
                self.reg_pc = address + 1;

                cycle_time = 6;
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

                cycle_time = 3;
            }
            Opcode::Php => {
                // bit 4 and 5 must be set
                let status = self.reg_status | 0x30;
                self.push_stack(status);

                cycle_time = 3;
            }
            Opcode::Pla => {
                let result = self.pull_stack();

                // update flags
                self.set_flag_status(StatusFlag::Zero, result == 0);
                self.set_flag_status(StatusFlag::Negative, result & 0x80 != 0);

                self.reg_a = result;

                cycle_time = 4;
            }
            Opcode::Plp => {
                // Bits 4 and 5 should not be edited
                let old_status = self.reg_status & 0x30;
                self.reg_status = self.pull_stack() | old_status;

                cycle_time = 4;
            }
            Opcode::Sta => {
                assert!(is_operand_address);

                // STA has a special timing, these addressing modes add one cycle
                // in case of page cross, but if its STA, it will always add 1
                if instruction.addressing_mode.can_cross_page() && !did_page_cross {
                    cycle_time += 1;
                }

                self.write_bus(decoded_operand, self.reg_a);
            }
            Opcode::Stx => {
                assert!(is_operand_address);
                self.write_bus(decoded_operand, self.reg_x);
            }
            Opcode::Sty => {
                assert!(is_operand_address);
                self.write_bus(decoded_operand, self.reg_y);
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

            // Unofficial instructions
            Opcode::Slo => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Asl,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Ora,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                // its as if the page crossed, even if it did not
                let page_cross_increment =
                    (instruction.addressing_mode.can_cross_page() && !did_page_cross) as u8;
                cycle_time += 2 + page_cross_increment;
            }
            Opcode::Sre => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Lsr,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Eor,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                // its as if the page crossed, even if it did not
                let page_cross_increment =
                    (instruction.addressing_mode.can_cross_page() && !did_page_cross) as u8;
                cycle_time += 2 + page_cross_increment;
            }
            Opcode::Rla => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Rol,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::And,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                // its as if the page crossed, even if it did not
                let page_cross_increment =
                    (instruction.addressing_mode.can_cross_page() && !did_page_cross) as u8;
                cycle_time += 2 + page_cross_increment;
            }
            Opcode::Rra => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Ror,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Adc,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                // its as if the page crossed, even if it did not
                let page_cross_increment =
                    (instruction.addressing_mode.can_cross_page() && !did_page_cross) as u8;
                cycle_time += 2 + page_cross_increment;
            }
            Opcode::Isc => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Inc,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Sbc,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                // its as if the page crossed, even if it did not
                let page_cross_increment =
                    (instruction.addressing_mode.can_cross_page() && !did_page_cross) as u8;
                cycle_time += 2 + page_cross_increment;
            }
            Opcode::Dcp => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Dec,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Cmp,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                // its as if the page crossed, even if it did not
                let page_cross_increment =
                    (instruction.addressing_mode.can_cross_page() && !did_page_cross) as u8;
                cycle_time += 2 + page_cross_increment;
            }
            Opcode::Sax => {
                assert!(is_operand_address);
                self.write_bus(decoded_operand, self.reg_x & self.reg_a);
            }
            Opcode::Lax => {
                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Lda,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::Ldx,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;
            }
            Opcode::Anc => {
                assert!(instruction.addressing_mode == AddressingMode::Immediate);

                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::And,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                self.set_flag_status(
                    StatusFlag::Carry,
                    self.reg_status & StatusFlag::Negative as u8 != 0,
                );
            }
            Opcode::Alr => {
                assert!(instruction.addressing_mode == AddressingMode::Immediate);

                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::And,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: 0, // unused
                    opcode: Opcode::Lsr,
                    addressing_mode: AddressingMode::Accumulator,
                });
                self.cycles_to_wait = old_cycles_to_wait;
            }
            Opcode::Arr => {
                assert!(instruction.addressing_mode == AddressingMode::Immediate);

                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::And,
                    addressing_mode: instruction.addressing_mode,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: 0, // unused
                    opcode: Opcode::Ror,
                    addressing_mode: AddressingMode::Accumulator,
                });
                self.cycles_to_wait = old_cycles_to_wait;

                self.set_flag_status(StatusFlag::Carry, (self.reg_a >> 6) & 1 != 0);
                self.set_flag_status(
                    StatusFlag::Overflow,
                    ((self.reg_a >> 6) & 1) ^ ((self.reg_a >> 5) & 1) != 0,
                );
            }
            Opcode::Axs => {
                assert!(instruction.addressing_mode == AddressingMode::Immediate);

                let (result, overflow) =
                    (self.reg_x & self.reg_a).overflowing_sub(decoded_operand as u8);

                self.reg_x = self.load(result as u16, false);

                self.set_flag_status(StatusFlag::Carry, !overflow);
            }
            Opcode::Xaa => {
                assert!(instruction.addressing_mode == AddressingMode::Immediate);

                let old_cycles_to_wait = self.cycles_to_wait;
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: 0, // unused
                    opcode: Opcode::Txa,
                    addressing_mode: AddressingMode::Implied,
                });
                self.run_instruction(&Instruction {
                    opcode_byte: 0,
                    operand: instruction.operand,
                    opcode: Opcode::And,
                    addressing_mode: instruction.addressing_mode,
                });
                self.cycles_to_wait = old_cycles_to_wait;
            }
            Opcode::Ahx => {
                assert!(is_operand_address);

                let high_byte = (decoded_operand >> 8) as u8;

                self.write_bus(decoded_operand, self.reg_a & self.reg_x & high_byte);

                cycle_time += !did_page_cross as u8;
            }
            Opcode::Shy => {
                assert!(is_operand_address);

                let low_byte = decoded_operand & 0xFF;
                let high_byte = (decoded_operand >> 8) as u8;

                let value = self.reg_y & (high_byte + 1);

                self.write_bus((value as u16) << 8 | low_byte, value);

                cycle_time += !did_page_cross as u8;
            }
            Opcode::Shx => {
                assert!(is_operand_address);

                let low_byte = decoded_operand & 0xFF;
                let high_byte = (decoded_operand >> 8) as u8;

                let value = self.reg_x & (high_byte + 1);

                self.write_bus((value as u16) << 8 | low_byte, value);

                cycle_time += !did_page_cross as u8;
            }
            Opcode::Tas => {
                assert!(is_operand_address);

                let high_byte = (decoded_operand >> 8) as u8;

                self.reg_sp = self.reg_x & self.reg_a;

                self.write_bus(decoded_operand, self.reg_sp & high_byte);

                cycle_time += !did_page_cross as u8;
            }
            Opcode::Las => {
                assert!(is_operand_address);

                let value = self.read_bus(decoded_operand);

                //set the flags
                let result = self.load((value & self.reg_sp) as u16, false);

                self.reg_a = result;
                self.reg_x = result;
                self.reg_sp = result;
            }
            Opcode::Kil => {
                // TODO: implement halt
                println!("KIL instruction executed, should halt....");
            }
        };

        // minus this cycle
        self.cycles_to_wait += cycle_time - 1;

        state
    }
}
