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
            Opcode::Adc => Self::run_instruction_adc,
            Opcode::And => Self::run_instruction_and,
            Opcode::Asl => Self::run_instruction_asl,
            Opcode::Eor => Self::run_instruction_eor,
            Opcode::Lsr => Self::run_instruction_lsr,
            Opcode::Ora => Self::run_instruction_ora,
            Opcode::Rol => Self::run_instruction_rol,
            Opcode::Ror => Self::run_instruction_ror,
            Opcode::Sbc => Self::run_instruction_sbc,
            Opcode::Bit => Self::run_instruction_bit,
            Opcode::Cmp => Self::run_instruction_cmp,
            Opcode::Cpx => Self::run_instruction_cpx,
            Opcode::Cpy => Self::run_instruction_cpy,
            Opcode::Brk => Self::run_instruction_brk,
            Opcode::Bcc => Self::run_instruction_bcc,
            Opcode::Bcs => Self::run_instruction_bcs,
            Opcode::Beq => Self::run_instruction_beq,
            Opcode::Bmi => Self::run_instruction_bmi,
            Opcode::Bne => Self::run_instruction_bne,
            Opcode::Bpl => Self::run_instruction_bpl,
            Opcode::Bvc => Self::run_instruction_bvc,
            Opcode::Bvs => Self::run_instruction_bvs,
            Opcode::Dec => Self::run_instruction_dec,
            Opcode::Inc => Self::run_instruction_inc,
            Opcode::Clc => Self::run_instruction_clc,
            Opcode::Cld => Self::run_instruction_cld,
            Opcode::Cli => Self::run_instruction_cli,
            Opcode::Clv => Self::run_instruction_clv,
            Opcode::Sec => Self::run_instruction_sec,
            Opcode::Sed => Self::run_instruction_sed,
            Opcode::Sei => Self::run_instruction_sei,
            Opcode::Jmp => Self::run_instruction_jmp,
            Opcode::Jsr => Self::run_instruction_jsr,
            Opcode::Rti => Self::run_instruction_rti,
            Opcode::Rts => Self::run_instruction_rts,
            Opcode::Lda => Self::run_instruction_lda,
            Opcode::Ldx => Self::run_instruction_ldx,
            Opcode::Ldy => Self::run_instruction_ldy,
            Opcode::Nop => Self::run_instruction_nop,
            Opcode::Dex => Self::run_instruction_dex,
            Opcode::Dey => Self::run_instruction_dey,
            Opcode::Inx => Self::run_instruction_inx,
            Opcode::Iny => Self::run_instruction_iny,
            Opcode::Tax => Self::run_instruction_tax,
            Opcode::Tay => Self::run_instruction_tay,
            Opcode::Txa => Self::run_instruction_txa,
            Opcode::Tya => Self::run_instruction_tya,
            Opcode::Pha => Self::run_instruction_pha,
            Opcode::Php => Self::run_instruction_php,
            Opcode::Pla => Self::run_instruction_pla,
            Opcode::Plp => Self::run_instruction_plp,
            Opcode::Sta => Self::run_instruction_sta,
            Opcode::Stx => Self::run_instruction_stx,
            Opcode::Sty => Self::run_instruction_sty,
            Opcode::Tsx => Self::run_instruction_tsx,
            Opcode::Txs => Self::run_instruction_txs,
        };

        handler(
            self,
            self.decode_operand(instruction),
            instruction.is_operand_address(),
        );
    }

    // TODO: fill instructions code
    fn run_instruction_adc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_asl(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_and(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_eor(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_lsr(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_ora(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_rol(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_ror(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_sbc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bit(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_cmp(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_cpx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_cpy(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_brk(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bcc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bcs(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_beq(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bmi(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bne(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bpl(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bvc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_bvs(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_dec(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_inc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_clc(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_cld(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_cli(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_clv(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_sec(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_sed(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_sei(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_jmp(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_jsr(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_rti(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_rts(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_lda(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_ldx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_ldy(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_nop(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_dex(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_dey(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_inx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_iny(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_tax(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_tay(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_txa(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_tya(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_pha(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_php(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_pla(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_plp(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_sta(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_stx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_sty(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_tsx(&mut self, operand_decoded: u16, is_operand_address: bool) {}
    fn run_instruction_txs(&mut self, operand_decoded: u16, is_operand_address: bool) {}
}
