use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::core::memory::*;
use crate::core::ports::{InputPin, OutputPin};
use crate::core::*;

pub struct C6502 {
    pc: u16,
    ac: u8,
    x: u8,
    y: u8,
    p: u8,
    sp: u8,
    cycle: usize,
    opcode: u8,
    value: u8,
    addr: u16,
    extra_addr: u16,
    memory: Memory,
    state: CpuState,

    phi0_in: InputPin,
    phi1_out: OutputPin,
    phi2_out: OutputPin,
}

impl fmt::Debug for C6502 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "OP: {:02X} PC: {:04X} AC: {:02X} X: {:02X} Y: {:02X} P: {:02X} SP: {:02X}",
            self.opcode, self.pc, self.ac, self.x, self.y, self.p, self.sp
        )
    }
}

impl C6502 {
    pub const SR_NEGATIVE: u8 = 0b10000000;
    pub const SR_OVERFLOW: u8 = 0b01000000;
    pub const SR_UNUSED: u8 = 0b00100000;
    pub const SR_BREAK: u8 = 0b00010000;
    pub const SR_BCD: u8 = 0b00001000;
    pub const SR_INTERRUPT_MASK: u8 = 0b00000100;
    pub const SR_ZERO: u8 = 0b00000010;
    pub const SR_CARRY: u8 = 0b00000001;

    pub const STACK_BASE: u16 = 0x0100;
    pub const NMI_VECTOR: u16 = 0xFFFA;
    pub const RESET_VECTOR: u16 = 0xFFFC;
    pub const IRQ_VECTOR: u16 = 0xFFFE;

    pub fn new(memory: &Memory) -> Self {
        Self {
            pc: 0x00FF,
            ac: 0xAA,
            x: 0x00,
            y: 0x00,
            p: 0x00,
            sp: 0xFF,
            cycle: 1,
            opcode: 0x00,
            value: 0x00,
            addr: 0x0000,
            extra_addr: 0x0000,
            state: CpuState::Off,
            memory: memory.clone(),
            phi0_in: InputPin::new(),
            phi1_out: OutputPin::new(),
            phi2_out: OutputPin::new(),
        }
    }

    pub fn state(&self) -> CpuState {
        self.state
    }

    pub fn phi0_in(&mut self) -> &mut InputPin {
        &mut self.phi0_in
    }
    
    pub fn phi1_out(&mut self) -> &mut OutputPin {
        &mut self.phi1_out
    }

    pub fn phi2_out(&mut self) -> &mut OutputPin {
        &mut self.phi2_out
    }

    pub fn reset(&mut self) {
        // TODO: Need to implement a more realistic reset mechanism.
        self.state = CpuState::Resetting;
        self.cycle = 1;
    }

    pub fn set_irq(&mut self) {
        if self.p & C6502::SR_INTERRUPT_MASK == 0 {
            unimplemented!();
        }
    }

    pub fn set_nmi(&mut self) {
        unimplemented!();
    }

    pub fn step(&mut self) -> CpuAction {
        match self.state {
            CpuState::Running => {
                // Fetch an opcode if we don't have one.
                if self.cycle == 1 {
                    self.opcode = self.read_pc_byte();
                    self.pc += 1;
                    self.cycle = 2;
                    return CpuAction::Continue;
                }

                let next_action = match self.opcode {
                    0x00 => self.do_brk(),
                    0x01 => self.do_op_indexed_indirect(Op::Read(Self::op_ora)),
                    0x04 => self.do_op_zeropage(Op::Implied(Self::op_nop)),
                    0x05 => self.do_op_zeropage(Op::Read(Self::op_ora)),
                    0x06 => self.do_op_zeropage(Op::ReadWrite(Self::op_asl)),
                    0x08 => self.do_php(),
                    0x09 => self.do_op_immed(Op::Read(Self::op_ora)),
                    0x0A => self.do_op_ac(Op::ReadWrite(Self::op_asl)),
                    0x0C => self.do_op_abs(Op::Implied(Self::op_nop)),
                    0x0D => self.do_op_abs(Op::Read(Self::op_ora)),
                    0x0E => self.do_op_abs(Op::ReadWrite(Self::op_asl)),
                    0x10 => self.do_branch(Self::br_bpl),
                    0x11 => self.do_op_indirect_indexed(Op::Read(Self::op_ora)),
                    0x14 => self.do_op_zeropage_x(Op::Implied(Self::op_nop)),
                    0x15 => self.do_op_zeropage_x(Op::Read(Self::op_ora)),
                    0x16 => self.do_op_zeropage_x(Op::ReadWrite(Self::op_asl)),
                    0x18 => self.do_op_implied(Op::Implied(Self::op_clc)),
                    0x19 => self.do_op_abs_y(Op::Read(Self::op_ora)),
                    0x1A => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0x1C => self.do_op_abs_x(Op::Implied(Self::op_nop)),
                    0x1D => self.do_op_abs_x(Op::Read(Self::op_ora)),
                    0x1E => self.do_op_abs_x(Op::ReadWrite(Self::op_asl)),
                    0x20 => self.do_jsr(),
                    0x21 => self.do_op_indexed_indirect(Op::Read(Self::op_and)),
                    0x24 => self.do_op_zeropage(Op::Read(Self::op_bit)),
                    0x25 => self.do_op_zeropage(Op::Read(Self::op_and)),
                    0x26 => self.do_op_zeropage(Op::ReadWrite(Self::op_rol)),
                    0x28 => self.do_plp(),
                    0x29 => self.do_op_immed(Op::Read(Self::op_and)),
                    0x2A => self.do_op_ac(Op::ReadWrite(Self::op_rol)),
                    0x2C => self.do_op_abs(Op::Read(Self::op_bit)),
                    0x2D => self.do_op_abs(Op::Read(Self::op_and)),
                    0x2E => self.do_op_abs(Op::ReadWrite(Self::op_rol)),
                    0x30 => self.do_branch(Self::br_bmi),
                    0x31 => self.do_op_indirect_indexed(Op::Read(Self::op_and)),
                    0x34 => self.do_op_zeropage_x(Op::Implied(Self::op_nop)),
                    0x35 => self.do_op_zeropage_x(Op::Read(Self::op_and)),
                    0x36 => self.do_op_zeropage_x(Op::ReadWrite(Self::op_rol)),
                    0x38 => self.do_op_implied(Op::Implied(Self::op_sec)),
                    0x39 => self.do_op_abs_y(Op::Read(Self::op_and)),
                    0x3A => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0x3C => self.do_op_abs_x(Op::Implied(Self::op_nop)),
                    0x3D => self.do_op_abs_x(Op::Read(Self::op_and)),
                    0x3E => self.do_op_abs_x(Op::ReadWrite(Self::op_rol)),
                    0x40 => self.do_rti(),
                    0x41 => self.do_op_indexed_indirect(Op::Read(Self::op_eor)),
                    0x44 => self.do_op_zeropage(Op::Implied(Self::op_nop)),
                    0x45 => self.do_op_zeropage(Op::Read(Self::op_eor)),
                    0x46 => self.do_op_zeropage(Op::ReadWrite(Self::op_lsr)),
                    0x48 => self.do_pha(),
                    0x49 => self.do_op_immed(Op::Read(Self::op_eor)),
                    0x4A => self.do_op_ac(Op::ReadWrite(Self::op_lsr)),
                    0x4C => self.do_jmp_abs(),
                    0x4D => self.do_op_abs(Op::Read(Self::op_eor)),
                    0x4E => self.do_op_abs(Op::ReadWrite(Self::op_lsr)),
                    0x50 => self.do_branch(Self::br_bvc),
                    0x51 => self.do_op_indirect_indexed(Op::Read(Self::op_eor)),
                    0x54 => self.do_op_zeropage_x(Op::Implied(Self::op_nop)),
                    0x55 => self.do_op_zeropage_x(Op::Read(Self::op_eor)),
                    0x56 => self.do_op_zeropage_x(Op::ReadWrite(Self::op_lsr)),
                    0x58 => self.do_op_implied(Op::Implied(Self::op_cli)),
                    0x59 => self.do_op_abs_y(Op::Read(Self::op_eor)),
                    0x5A => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0x5C => self.do_op_abs_x(Op::Implied(Self::op_nop)),
                    0x5D => self.do_op_abs_x(Op::Read(Self::op_eor)),
                    0x5E => self.do_op_abs_x(Op::ReadWrite(Self::op_lsr)),
                    0x60 => self.do_rts(),
                    0x61 => self.do_op_indexed_indirect(Op::Read(Self::op_adc)),
                    0x64 => self.do_op_zeropage(Op::Implied(Self::op_nop)),
                    0x65 => self.do_op_zeropage(Op::Read(Self::op_adc)),
                    0x66 => self.do_op_zeropage(Op::ReadWrite(Self::op_ror)),
                    0x68 => self.do_pla(),
                    0x69 => self.do_op_immed(Op::Read(Self::op_adc)),
                    0x6A => self.do_op_ac(Op::ReadWrite(Self::op_ror)),
                    0x6C => self.do_jmp_abs_indirect(),
                    0x6D => self.do_op_abs(Op::Read(Self::op_adc)),
                    0x6E => self.do_op_abs(Op::ReadWrite(Self::op_ror)),
                    0x70 => self.do_branch(Self::br_bvs),
                    0x71 => self.do_op_indirect_indexed(Op::Read(Self::op_adc)),
                    0x74 => self.do_op_zeropage_x(Op::Implied(Self::op_nop)),
                    0x75 => self.do_op_zeropage_x(Op::Read(Self::op_adc)),
                    0x76 => self.do_op_zeropage_x(Op::ReadWrite(Self::op_ror)),
                    0x78 => self.do_op_implied(Op::Implied(Self::op_sei)),
                    0x79 => self.do_op_abs_y(Op::Read(Self::op_adc)),
                    0x7A => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0x7C => self.do_op_abs_x(Op::Implied(Self::op_nop)),
                    0x7D => self.do_op_abs_x(Op::Read(Self::op_adc)),
                    0x7E => self.do_op_abs_x(Op::ReadWrite(Self::op_ror)),
                    0x80 => self.do_op_immed(Op::Implied(Self::op_nop)),
                    0x81 => self.do_op_indexed_indirect(Op::Write(Self::op_sta)),
                    0x82 => self.do_op_immed(Op::Implied(Self::op_nop)),
                    0x84 => self.do_op_zeropage(Op::Write(Self::op_sty)),
                    0x85 => self.do_op_zeropage(Op::Write(Self::op_sta)),
                    0x86 => self.do_op_zeropage(Op::Write(Self::op_stx)),
                    0x88 => self.do_op_implied(Op::Implied(Self::op_dey)),
                    0x89 => self.do_op_immed(Op::Implied(Self::op_nop)),
                    0x8A => self.do_op_implied(Op::Implied(Self::op_txa)),
                    0x8C => self.do_op_abs(Op::Write(Self::op_sty)),
                    0x8D => self.do_op_abs(Op::Write(Self::op_sta)),
                    0x8E => self.do_op_abs(Op::Write(Self::op_stx)),
                    0x90 => self.do_branch(Self::br_bcc),
                    0x91 => self.do_op_indirect_indexed(Op::Write(Self::op_sta)),
                    0x94 => self.do_op_zeropage_x(Op::Write(Self::op_sty)),
                    0x95 => self.do_op_zeropage_x(Op::Write(Self::op_sta)),
                    0x96 => self.do_op_zeropage_y(Op::Write(Self::op_stx)),
                    0x98 => self.do_op_implied(Op::Implied(Self::op_tya)),
                    0x99 => self.do_op_abs_y(Op::Write(Self::op_sta)),
                    0x9A => self.do_op_implied(Op::Implied(Self::op_txs)),
                    0x9D => self.do_op_abs_x(Op::Write(Self::op_sta)),
                    0xA0 => self.do_op_immed(Op::Read(Self::op_ldy)),
                    0xA1 => self.do_op_indexed_indirect(Op::Read(Self::op_lda)),
                    0xA2 => self.do_op_immed(Op::Read(Self::op_ldx)),
                    0xA4 => self.do_op_zeropage(Op::Read(Self::op_ldy)),
                    0xA5 => self.do_op_zeropage(Op::Read(Self::op_lda)),
                    0xA6 => self.do_op_zeropage(Op::Read(Self::op_ldx)),
                    0xA8 => self.do_op_implied(Op::Implied(Self::op_tay)),
                    0xA9 => self.do_op_immed(Op::Read(Self::op_lda)),
                    0xAA => self.do_op_implied(Op::Implied(Self::op_tax)),
                    0xAC => self.do_op_abs(Op::Read(Self::op_ldy)),
                    0xAD => self.do_op_abs(Op::Read(Self::op_lda)),
                    0xAE => self.do_op_abs(Op::Read(Self::op_ldx)),
                    0xB0 => self.do_branch(Self::br_bcs),
                    0xB1 => self.do_op_indirect_indexed(Op::Read(Self::op_lda)),
                    0xB4 => self.do_op_zeropage_x(Op::Read(Self::op_ldy)),
                    0xB5 => self.do_op_zeropage_x(Op::Read(Self::op_lda)),
                    0xB6 => self.do_op_zeropage_y(Op::Read(Self::op_ldx)),
                    0xB8 => self.do_op_implied(Op::Implied(Self::op_clv)),
                    0xB9 => self.do_op_abs_y(Op::Read(Self::op_lda)),
                    0xBA => self.do_op_implied(Op::Implied(Self::op_tsx)),
                    0xBC => self.do_op_abs_x(Op::Read(Self::op_ldy)),
                    0xBD => self.do_op_abs_x(Op::Read(Self::op_lda)),
                    0xBE => self.do_op_abs_y(Op::Read(Self::op_ldx)),
                    0xC0 => self.do_op_immed(Op::Read(Self::op_cpy)),
                    0xC1 => self.do_op_indexed_indirect(Op::Read(Self::op_cmp)),
                    0xC2 => self.do_op_immed(Op::Implied(Self::op_nop)),
                    0xC4 => self.do_op_zeropage(Op::Read(Self::op_cpy)),
                    0xC5 => self.do_op_zeropage(Op::Read(Self::op_cmp)),
                    0xC6 => self.do_op_zeropage(Op::ReadWrite(Self::op_dec)),
                    0xC8 => self.do_op_implied(Op::Implied(Self::op_iny)),
                    0xC9 => self.do_op_immed(Op::Read(Self::op_cmp)),
                    0xCA => self.do_op_implied(Op::Implied(Self::op_dex)),
                    0xCC => self.do_op_abs(Op::Read(Self::op_cpy)),
                    0xCD => self.do_op_abs(Op::Read(Self::op_cmp)),
                    0xCE => self.do_op_abs(Op::ReadWrite(Self::op_dec)),
                    0xD0 => self.do_branch(Self::br_bne),
                    0xD1 => self.do_op_indirect_indexed(Op::Read(Self::op_cmp)),
                    0xD4 => self.do_op_zeropage_x(Op::Implied(Self::op_nop)),
                    0xD5 => self.do_op_zeropage_x(Op::Read(Self::op_cmp)),
                    0xD6 => self.do_op_zeropage_x(Op::ReadWrite(Self::op_dec)),
                    0xD8 => self.do_op_implied(Op::Implied(Self::op_cld)),
                    0xD9 => self.do_op_abs_y(Op::Read(Self::op_cmp)),
                    0xDA => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0xDC => self.do_op_abs_x(Op::Implied(Self::op_nop)),
                    0xDD => self.do_op_abs_x(Op::Read(Self::op_cmp)),
                    0xDE => self.do_op_abs_x(Op::ReadWrite(Self::op_dec)),
                    0xE0 => self.do_op_immed(Op::Read(Self::op_cpx)),
                    0xE1 => self.do_op_indexed_indirect(Op::Read(Self::op_sbc)),
                    0xE2 => self.do_op_immed(Op::Implied(Self::op_nop)),
                    0xE4 => self.do_op_zeropage(Op::Read(Self::op_cpx)),
                    0xE5 => self.do_op_zeropage(Op::Read(Self::op_sbc)),
                    0xE6 => self.do_op_zeropage(Op::ReadWrite(Self::op_inc)),
                    0xE8 => self.do_op_implied(Op::Implied(Self::op_inx)),
                    0xE9 => self.do_op_immed(Op::Read(Self::op_sbc)),
                    0xEA => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0xEC => self.do_op_abs(Op::Read(Self::op_cpx)),
                    0xED => self.do_op_abs(Op::Read(Self::op_sbc)),
                    0xEE => self.do_op_abs(Op::ReadWrite(Self::op_inc)),
                    0xF0 => self.do_branch(Self::br_beq),
                    0xF1 => self.do_op_indirect_indexed(Op::Read(Self::op_sbc)),
                    0xF4 => self.do_op_zeropage_x(Op::Implied(Self::op_nop)),
                    0xF5 => self.do_op_zeropage_x(Op::Read(Self::op_sbc)),
                    0xF6 => self.do_op_zeropage_x(Op::ReadWrite(Self::op_inc)),
                    0xF8 => self.do_op_implied(Op::Implied(Self::op_sed)),
                    0xF9 => self.do_op_abs_y(Op::Read(Self::op_sbc)),
                    0xFA => self.do_op_implied(Op::Implied(Self::op_nop)),
                    0xFC => self.do_op_abs_x(Op::Implied(Self::op_nop)),
                    0xFD => self.do_op_abs_x(Op::Read(Self::op_sbc)),
                    0xFE => self.do_op_abs_x(Op::ReadWrite(Self::op_inc)),
                    _ => panic!("Illegal instruction ${:02X} at ${:04X}", self.opcode, self.pc - 1),
                };

                match next_action {
                    CpuAction::Continue => {
                        self.cycle += 1;
                    },
                    CpuAction::Complete => {
                        self.cycle = 1;
                    },
                    CpuAction::CompleteAndFetch => {
                        // For instructions that don't write to memory, we need to pipeline the next
                        // opcode during this cycle.
                        self.opcode = self.read_pc_byte();
                        self.pc += 1;
                        self.cycle = 2;
                    },
                }

                next_action
            },

            CpuState::Off => CpuAction::Continue,

            CpuState::Resetting => {
                // Go through next cycle of reset sequence, until completed.
                if self.do_reset_sequence() {
                    self.state = CpuState::Running;
                    self.cycle = 1;
                    CpuAction::Complete
                } else {
                    self.cycle += 1;
                    CpuAction::Continue
                }
            },
        }
    }

    fn read_byte(&self, addr: u16) -> u8 {
        self.memory.read_byte(addr)
    }

    fn read_pc_byte(&self) -> u8 {
        self.read_byte(self.pc)
    }

    fn push_byte(&mut self, value: u8) {
        if self.sp == 0 {
            panic!("Stack overflow");
        }
        self.memory.write_byte(Self::STACK_BASE + self.sp as u16, value);
        self.sp -= 1;
    }

    fn incr_stack(&mut self) {
        if self.sp == 0xff {
            panic!("Stack underflow");
        }
        self.sp += 1;
    }

    fn read_stack_byte(&mut self) -> u8 {
        self.memory.read_byte(Self::STACK_BASE + self.sp as u16)
    }

    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory.write_byte(addr, value);
    }

    /// Go through reset cycle.
    ///
    /// The first few steps are just for simulation accuracy,
    /// but the key parts start at step 7, where we load the PC from the reset vector ($FFFC).
    ///
    fn do_reset_sequence(&mut self) -> bool {
        match self.cycle {
            1 => self.sp = 0x00,
            2 | 3 => {},
            4 => self.sp = 0xFF,
            5 => self.sp = 0xFE,
            6 => self.sp = 0xFD,
            7 => set_lo_byte!(&mut self.pc, self.read_byte(Self::RESET_VECTOR)),
            8 => set_hi_byte!(&mut self.pc, self.read_byte(Self::RESET_VECTOR + 1)),
            _ => unreachable!(),
        }
        self.cycle == 8
    }

    fn do_brk(&mut self) -> CpuAction {
        // TODO: Need to figure out when to set the Interrupt mask.
        match self.cycle {
            2 => {
                //self.read_pc_byte();
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                self.push_byte(hi_byte!(self.pc));
                CpuAction::Continue
            },
            4 => {
                self.push_byte(lo_byte!(self.pc));
                CpuAction::Continue
            },
            5 => {
                self.push_byte(self.p | Self::SR_BREAK | Self::SR_UNUSED);
                CpuAction::Continue
            },
            6 => {
                set_lo_byte!(&mut self.pc, self.read_byte(Self::IRQ_VECTOR));
                CpuAction::Continue
            },
            7 => {
                set_hi_byte!(&mut self.pc, self.read_byte(Self::IRQ_VECTOR + 1));
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_rti(&mut self) -> CpuAction {
        // TODO: Need to figure out when to clear the Interrupt mask.
        match self.cycle {
            2 => {
                // self.read_pc_byte();
                CpuAction::Continue
            },
            3 => {
                self.incr_stack();
                CpuAction::Continue
            },
            4 => {
                self.p = self.read_stack_byte() & !(Self::SR_BREAK | Self::SR_UNUSED);
                self.incr_stack();
                CpuAction::Continue
            },
            5 => {
                set_lo_byte!(&mut self.pc, self.read_stack_byte());
                self.incr_stack();
                CpuAction::Continue
            },
            6 => {
                set_hi_byte!(&mut self.pc, self.read_stack_byte());
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_pha(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                // self.read_pc_byte()
                CpuAction::Continue
            },
            3 => {
                self.push_byte(self.ac);
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_php(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                // self.read_pc_byte()
                CpuAction::Continue
            },
            3 => {
                self.push_byte(self.p | Self::SR_BREAK | Self::SR_UNUSED);
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_jsr(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                // The purpose of this cycle is unknown.
                CpuAction::Continue
            },
            4 => {
                self.push_byte(hi_byte!(self.pc));
                CpuAction::Continue
            },
            5 => {
                self.push_byte(lo_byte!(self.pc));
                CpuAction::Continue
            },
            6 => {
                set_hi_byte!(&mut self.addr, self.read_byte(self.pc));
                self.pc = self.addr;
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_rts(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                // self.read_pc_byte()
                CpuAction::Continue
            },
            3 => {
                self.incr_stack();
                CpuAction::Continue
            },
            4 => {
                set_lo_byte!(&mut self.pc, self.read_stack_byte());
                self.incr_stack();
                CpuAction::Continue
            },
            5 => {
                set_hi_byte!(&mut self.pc, self.read_stack_byte());
                CpuAction::Continue
            },
            6 => {
                self.pc += 1;
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_pla(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                // self.read_pc_byte()
                CpuAction::Continue
            },
            3 => {
                self.incr_stack();
                CpuAction::Continue
            },
            4 => {
                self.ac = self.read_stack_byte();
                self.set_nz(self.ac);
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    fn do_plp(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                // self.read_pc_byte()
                CpuAction::Continue
            },
            3 => {
                self.incr_stack();
                CpuAction::Continue
            },
            4 => {
                self.p = self.read_stack_byte() & !(Self::SR_BREAK | Self::SR_UNUSED);
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    /// Execute an absolute jump.
    ///
    /// The bytes for the instruction are `JMP LL HH`.
    ///
    /// The operand is a 16-bit absolute address (`$HHLL`).
    ///
    /// This instruction takes 3 cycles.
    ///
    fn do_jmp_abs(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                self.addr |= (self.read_pc_byte() as u16) << 8;
                self.pc = self.addr;
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    /// Execute an absolute indirect addressed jump.
    ///
    /// The bytes for the instruction are `JMP LL HH`.
    ///
    /// The operand is a 16-bit address (`$HHLL`) pointing to the actual jump address.
    ///
    /// If the operand points to the last byte of a page, the high bits of the jump address
    /// will be taken from location 0 of the same page, not the next physical byte (which is
    /// on the next page).
    ///
    /// This instruction takes 5 cycles.
    ///
    fn do_jmp_abs_indirect(&mut self) -> CpuAction {
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                set_hi_byte!(&mut self.addr, self.read_pc_byte());
                self.pc += 1;
                CpuAction::Continue
            },
            4 => {
                self.extra_addr = self.read_byte(self.addr) as u16;
                CpuAction::Continue
            },
            5 => {
                self.pc = self.extra_addr;
                set_hi_byte!(&mut self.pc, self.read_byte(self.addr & 0xFF00 | ((self.addr + 1) & 0xFF)));
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    /// Execute a branch on the result of a condition.
    ///
    /// The bytes for the instruction are `<branch-opcode> LL`.
    ///
    /// The operand is a relative offset.
    ///
    /// This instruction takes 2-4 cycles: 2 if there is no branch, 3 if there is
    /// a branch, and an extra cycle if the branch goes to a different page.
    ///
    fn do_branch(&mut self, test: fn(&C6502) -> bool) -> CpuAction {
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as i8 as i16 as u16;
                self.pc += 1;
                if test(self) {
                    CpuAction::Continue
                } else {
                    CpuAction::Complete
                }
            },
            3 => {
                self.addr = self.pc.wrapping_add(self.addr);
                set_lo_byte!(&mut self.pc, (self.addr & 0xFF) as u8);
                if self.pc == self.addr {
                    CpuAction::Complete
                } else {
                    CpuAction::Continue
                }
            },
            4 => {
                self.pc = self.addr;
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    /// Branch test for a branch on a positive value.
    ///
    fn br_bpl(&self) -> bool {
        self.p & Self::SR_NEGATIVE == 0
    }

    /// Branch test for a branch on a negative value.
    ///
    fn br_bmi(&self) -> bool {
        self.p & Self::SR_NEGATIVE != 0
    }

    /// Branch test for a branch on the overflow bit being clear.
    ///
    fn br_bvc(&self) -> bool {
        self.p & Self::SR_OVERFLOW == 0
    }

    /// Branch test for a branch on the overflow bit being set.
    ///
    fn br_bvs(&self) -> bool {
        self.p & Self::SR_OVERFLOW != 0
    }

    /// Branch test for a branch on the carry bit being clear.
    ///
    fn br_bcc(&self) -> bool {
        self.p & Self::SR_CARRY == 0
    }

    /// Branch test for a branch on the carry bit being set.
    ///
    fn br_bcs(&self) -> bool {
        self.p & Self::SR_CARRY != 0
    }

    /// Branch test for a branch on the zero bit being clear.
    ///
    fn br_bne(&self) -> bool {
        self.p & Self::SR_ZERO == 0
    }

    /// Branch test for a branch on the zero bit being set.
    ///
    fn br_beq(&self) -> bool {
        self.p & Self::SR_ZERO != 0
    }

    fn op_clc(&mut self) {
        self.p &= !Self::SR_CARRY;
    }

    fn op_cli(&mut self) {
        self.p &= !Self::SR_INTERRUPT_MASK;
    }

    fn op_clv(&mut self) {
        self.p &= !Self::SR_OVERFLOW;
    }

    fn op_cld(&mut self) {
        self.p &= !Self::SR_BCD;
    }

    fn op_sei(&mut self) {
        self.p |= Self::SR_INTERRUPT_MASK;
    }

    fn op_dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.set_nz(self.x);
    }

    fn op_dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.set_nz(self.y);
    }

    fn op_inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.set_nz(self.x);
    }

    fn op_iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.set_nz(self.y);
    }

    fn op_txa(&mut self) {
        self.ac = self.x;
        self.set_nz(self.ac);
    }

    fn op_tya(&mut self) {
        self.ac = self.y;
        self.set_nz(self.ac);
    }

    fn op_tax(&mut self) {
        self.x = self.ac;
        self.set_nz(self.x);
    }

    fn op_tay(&mut self) {
        self.y = self.ac;
        self.set_nz(self.y);
    }

    fn op_txs(&mut self) {
        self.sp = self.x;
    }

    fn op_tsx(&mut self) {
        self.x = self.sp;
        self.set_nz(self.x);
    }

    fn op_nop(&mut self) {
        // Do nothing
    }

    fn op_sec(&mut self) {
        self.p |= Self::SR_CARRY;
    }

    fn op_sed(&mut self) {
        self.p |= Self::SR_BCD;
    }

    /// Do an operation with immediate addressing.
    ///
    /// The bytes for the instruction are `<opcode> BB`.
    ///
    /// The operand is an 8-bit value #$BB. By necessity, the operation must be a
    /// read-only operation.
    ///
    /// This instruction takes 3 cycles, the last of which also fetches
    /// the next instruction.
    ///
    fn do_op_immed(&mut self, op: Op) -> CpuAction {
        match self.cycle {
            2 => {
                self.value = self.read_pc_byte();
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                match op {
                    Op::Read(f) => f(self, self.value),
                    Op::Implied(f) => f(self),
                    _ => unreachable!(),
                }
                CpuAction::CompleteAndFetch
            },
            _ => unreachable!(),
        }
    }

    /// Do an operation with accumulator addressing.
    ///
    /// The bytes for the instruction are `<opcode>`.
    ///
    /// The operand is implied to be the accumulator. By necessity, the operation must be a
    /// read-only operation.
    ///
    /// This instruction takes 3 cycles, the last of which also fetches
    /// the next instruction.
    ///
    fn do_op_ac(&mut self, op: Op) -> CpuAction {
        if let Op::ReadWrite(op) = op {
            match self.cycle {
                2 => {
                    // self.read_pc_byte();
                    CpuAction::Continue
                },
                3 => {
                    self.ac = op(self, self.ac);
                    CpuAction::CompleteAndFetch
                },
                _ => unreachable!(),
            }
        } else {
            unreachable!();
        }
    }

    /// Do an operation with implied addressing.
    ///
    /// The bytes for the instruction are `<opcode>`.
    ///
    /// The operand is implied by the instruction. Usually, the instruction
    /// sets or clears some register flags.
    ///
    /// This instruction takes 3 cycles, the last of which also fetches
    /// the next instruction.
    ///
    fn do_op_implied(&mut self, op: Op) -> CpuAction {
        if let Op::Implied(op) = op {
            match self.cycle {
                2 => {
                    // self.read_pc_byte();
                    CpuAction::Continue
                },
                3 => {
                    op(self);
                    CpuAction::CompleteAndFetch
                },
                _ => unreachable!(),
            }
        } else {
            unreachable!();
        }
    }

    /// Do an operation with zero-page addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL`.
    ///
    /// The operand is an 8-bit zero page address, and the effective address is
    /// $00LL.
    ///
    /// This instruction takes between 3 and 5 cycles, depending on the operation
    /// (see `C6502::do_op`).
    ///
    fn do_op_zeropage(&mut self, op: Op) -> CpuAction {
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            _ => self.do_op(op, 3),
        }
    }

    /// Do an operation with zero page, X-indexed addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL`.
    ///
    /// The operand is the value at the address calculated by incrementing $00LL
    /// by the value in the X register, _without carry_: if it would result in an
    /// effective address higher than $00FF, the address is wrapped back to a zero page address.
    ///
    /// This instruction takes between 4 and 6 cycles, depending on the operation
    /// (see `C6502::do_op`).
    ///
    fn do_op_zeropage_x(&mut self, op: Op) -> CpuAction {
        self.do_op_zeropage_indexed(op, self.x)
    }

    /// Do an operation with zero page, Y-indexed addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL`.
    ///
    /// The operand is the value at the address calculated by incrementing $00LL
    /// by the value in the Y register, _without carry_: if it would result in an
    /// effective address higher than $00FF, the address is wrapped back to a zero page address.
    ///
    /// This instruction takes between 4 and 6 cycles, depending on the operation
    /// (see `C6502::do_op`).
    ///
    fn do_op_zeropage_y(&mut self, op: Op) -> CpuAction {
        self.do_op_zeropage_indexed(op, self.y)
    }

    /// Do an operation with zero page, indexed addressing.
    ///
    /// This is a helper function for `C6502::do_op_zeropg_x` and `C6502::do_op_zeropg_y`.
    ///
    fn do_op_zeropage_indexed(&mut self, op: Op, offset: u8) -> CpuAction {
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                //self.read_byte(self.addr);
                self.addr = (self.addr + offset as u16) & 0xFF;
                CpuAction::Continue
            },
            _ => self.do_op(op, 4),
        }
    }

    /// Do an operation with X-indexed, indirect addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL`.
    ///
    /// The operand is a zero-page address $00LL. The effective address is formed by reading
    /// the values at $00LL+X and $00LL+X+1, computed _without carry_: if either location is
    /// greater than $FF, it is wrapped back to a zero page address.
    ///
    /// This instruction takes between 6 and 8 cycles, depending on the operation
    /// (see `C6502::do_op`).
    ///
    fn do_op_indexed_indirect(&mut self, op: Op) -> CpuAction {
        match self.cycle {
            2 => {
                self.extra_addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                // self.read_byte(self.extra_addr);
                self.extra_addr = (self.extra_addr + self.x as u16) & 0xFF;
                CpuAction::Continue
            },
            4 => {
                set_lo_byte!(&mut self.addr, self.read_byte(self.extra_addr));
                self.extra_addr = (self.extra_addr + 1) & 0xFF;
                CpuAction::Continue
            },
            5 => {
                set_hi_byte!(&mut self.addr, self.read_byte(self.extra_addr));
                CpuAction::Continue
            },
            _ => self.do_op(op, 6),
        }
    }

    /// Do an operation with indirect, Y-indexed addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL`.
    ///
    /// The operand is a zero-page address $00LL. The effective address is formed by reading
    /// the values at $00LL and $00LL+1, and then incremented by the value of the Y register,
    /// _with carry_: if either location is greater than $FF, it is wrapped back to a zero page address.  
    ///
    /// This instruction takes between 6 and 8 cycles, depending on the operation
    /// (see `C6502::do_op`), and on whether the effective address is on the next page.
    ///
    fn do_op_indirect_indexed(&mut self, op: Op) -> CpuAction {
        let is_read = op.is_read_or_implied();
        match self.cycle {
            2 => {
                self.extra_addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                self.addr = self.read_byte(self.extra_addr) as u16;
                CpuAction::Continue
            },
            4 => {
                let addr = self.addr + self.y as u16;
                set_lo_byte!(&mut self.addr, addr & 0xFF);
                set_hi_byte!(&mut self.addr, self.read_byte((self.extra_addr + 1) & 0xFF));
                self.extra_addr = addr & 0x100;
                CpuAction::Continue
            },
            5 => {
                if is_read && self.extra_addr == 0 {
                    self.do_op(op, 5)
                } else {
                    //self.read_byte(self.addr);
                    self.addr += self.extra_addr;
                    CpuAction::Continue
                }
            },
            _ => self.do_op(op, if is_read && self.extra_addr == 0 { 5 } else { 6 }),
        }
    }

    /// Do an operation with absolute addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL HH`.
    ///
    /// The operand is the value at the 16-bit absolute address $HHLL.
    ///
    /// This instruction takes between 4 and 6 cycles, depending on the operation
    /// (see `C6502::do_op`).
    ///
    fn do_op_abs(&mut self, op: Op) -> CpuAction {
        match self.cycle {
            2 => {
                set_lo_byte!(&mut self.addr, self.read_pc_byte());
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                set_hi_byte!(&mut self.addr, self.read_pc_byte());
                self.pc += 1;
                CpuAction::Continue
            },
            _ => self.do_op(op, 4),
        }
    }

    /// Do an operation with absolute, X-indexed addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL HH`.
    ///
    /// The operand is the value at the address calculate by incrementing the
    /// 16-bit absolute address $HHLL by the value in the X register, _with carry_.
    ///
    /// This instruction takes between 5 and 7 cycles, depending on the operation
    /// (see `C6502::do_op`), and on whether the effective address is on the next page.
    ///
    fn do_op_abs_x(&mut self, op: Op) -> CpuAction {
        self.do_op_abs_indexed(op, self.x)
    }

    /// Do an operation with absolute, Y-indexed addressing.
    ///
    /// The bytes for the instruction are `<opcode> LL HH`.
    ///
    /// The operand is the value at the address calculate by incrementing the
    /// 16-bit absolute address $HHLL by the value in the Y register, _with carry_.
    /// If incrementing would result in an address on the next page, an additional
    /// cycle is consumed to read from the correct page.
    ///
    /// This instruction takes between 5 and 7 cycles, depending on the operation
    /// (see `C6502::do_op`), and on whether the effective address is on the next page.
    ///
    fn do_op_abs_y(&mut self, op: Op) -> CpuAction {
        self.do_op_abs_indexed(op, self.y)
    }

    /// Do an operation with absolute, indexed addressing.
    ///
    /// This is a helper function for `C6502::do_op_abs_x` and `C6502::do_op_abs_y`.
    ///
    fn do_op_abs_indexed(&mut self, op: Op, offset: u8) -> CpuAction {
        let is_read = op.is_read_or_implied();
        match self.cycle {
            2 => {
                self.addr = self.read_pc_byte() as u16;
                self.pc += 1;
                CpuAction::Continue
            },
            3 => {
                let addr = self.addr + offset as u16;
                self.addr = addr & 0xFF;
                self.extra_addr = addr & 0x100;
                set_hi_byte!(&mut self.addr, self.read_pc_byte());
                self.pc += 1;
                CpuAction::Continue
            },
            4 => {
                if is_read && self.extra_addr == 0 {
                    self.do_op(op, 4)
                } else {
                    //self.read_byte(self.addr);
                    self.addr += self.extra_addr;
                    CpuAction::Continue
                }
            },
            _ => self.do_op(op, if is_read && self.extra_addr == 0 { 4 } else { 5 }),
        }
    }

    /// Perform an operation on an address that has been resolved.
    ///
    /// This function is called by various addressing-mode specific functions
    /// to do the actual work. When this function is called, `self.addr`
    /// contains the effective address.
    ///
    /// This function performs three kinds of operations, as specified by the `op`
    /// parameter.
    ///
    /// * Read-only operations (`Op::Read`) specify a function that accepts the 8-bit value
    ///   found at the address, and usually sets some registers based on that value.
    ///   The function returns nothing. Read-only operations take 2 cycles after the address
    ///   computation, the second of which also fetches the next instruction.
    /// * Write-only operations (`Op::Write`) specify a function that returns an 8-bit value,
    ///   usually the value of a register. The returned value is then written to the address.
    ///   Write-only operations take 1 cycle after the address computation.
    /// * Read-write operations (`Op::ReadWrite`) specify a function that accepts the 8-bit
    ///   value found at the address, and returns a modified 8-bit value. The function usually
    ///   sets some registers as well. The returned value is then written to the address.
    ///   Read-write operations take 3 cycles after the address computation.
    ///
    fn do_op(&mut self, op: Op, start_at: usize) -> CpuAction {
        match self.cycle - start_at + 1 {
            1 => match op {
                Op::Read(_) | Op::ReadWrite(_) => {
                    self.value = self.read_byte(self.addr);
                    CpuAction::Continue
                },
                Op::Implied(_) => CpuAction::Continue,
                Op::Write(f) => {
                    let result = f(self);
                    self.write_byte(self.addr, result);
                    CpuAction::Complete
                },
            },
            2 => match op {
                Op::Read(op) => {
                    op(self, self.value);
                    CpuAction::CompleteAndFetch
                },
                Op::Implied(f) => {
                    f(self);
                    CpuAction::CompleteAndFetch
                },
                Op::ReadWrite(op) => {
                    self.value = op(self, self.value);
                    CpuAction::Continue
                },
                _ => unreachable!(),
            },
            3 => {
                self.write_byte(self.addr, self.value);
                CpuAction::Complete
            },
            _ => unreachable!(),
        }
    }

    /// Perform a bitwise OR of the value with the accumulator, store the result
    /// in the accumulator, and set the zero and negative flags as appropriate.
    ///
    fn op_ora(&mut self, value: u8) {
        self.ac |= value;
        self.set_nz(self.ac);
    }

    /// Perform a bitwise AND of the value with the accumulator, store the result
    /// in the accumulator, and set the zero and negative flags as appropriate.
    ///
    fn op_and(&mut self, value: u8) {
        self.ac &= value;
        self.set_nz(self.ac);
    }

    /// Perform a bitwise XOR of the value with the accumulator, store the result
    /// in the accumulator, and set the zero and negative flags as appropriate.
    ///
    fn op_eor(&mut self, value: u8) {
        self.ac ^= value;
        self.set_nz(self.ac);
    }

    /// Shift the value left by one bit, and return the result, setting the carry,
    /// zero, and negative flags as appropriate.
    ///
    fn op_asl(&mut self, value: u8) -> u8 {
        let result = value << 1;
        self.set_carry(value & 0x80 != 0);
        self.set_nz(result);
        result
    }

    /// Shift the value right by one bit, and return the result, setting the carry,
    /// zero, and negative flags as appropriate.
    ///
    fn op_lsr(&mut self, value: u8) -> u8 {
        let result = value >> 1;
        self.set_carry(value & 0x01 != 0);
        self.set_nz(result);
        result
    }

    /// Decrement the value by one, and return the result, setting the
    /// zero and negative flags as appropriate.
    ///
    fn op_dec(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_nz(result);
        result
    }

    /// Decrement the value by one, and return the result, setting the
    /// zero and negative flags as appropriate.
    ///
    fn op_inc(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_nz(result);
        result
    }

    /// Tests bits in the value together with the accumulator. Sets the zero flag if
    /// the bitwise AND of the value and the accumulator is zero, and sets the negative
    /// and overflow flags from the same bits in the value.
    fn op_bit(&mut self, value: u8) {
        self.p = (self.p & !(Self::SR_NEGATIVE | Self::SR_OVERFLOW | Self::SR_ZERO))
            | (value & (Self::SR_NEGATIVE | Self::SR_OVERFLOW))
            | if (self.ac & value) == 0 { Self::SR_ZERO } else { 0 };
    }

    /// Shift the operand left by one bit, rotating in the current value of the carry
    /// flag into bit 0, and return the result, setting the carry, zero, and negative flags
    /// as appropriate.
    ///
    fn op_rol(&mut self, value: u8) -> u8 {
        let result = (value << 1) | if (self.p & Self::SR_CARRY) != 0 { 1 } else { 0 };
        self.set_carry(value & 0x80 != 0);
        self.set_nz(result);
        result
    }

    /// Shift the operand right by one bit, rotating in the current value of the carry
    /// flag into bit 0, and return the result, setting the carry, zero, and negative flags
    /// as appropriate.
    ///
    fn op_ror(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | if (self.p & Self::SR_CARRY) != 0 { 0x80 } else { 0 };
        self.set_carry(value & 0x01 != 0);
        self.set_nz(result);
        result
    }

    /// Loads the value into the accumulator, and sets the zero and negative flags as appropriate.
    ///
    fn op_lda(&mut self, value: u8) {
        self.ac = value;
        self.set_nz(self.ac);
    }

    /// Loads the value into the X register, and sets the zero and negative flags as appropriate.
    ///
    fn op_ldx(&mut self, value: u8) {
        self.x = value;
        self.set_nz(self.x);
    }

    /// Loads the value into the Y register, and sets the zero and negative flags as appropriate.
    ///
    fn op_ldy(&mut self, value: u8) {
        self.y = value;
        self.set_nz(self.y);
    }

    /// Adds the value to the accumulator, setting the zero, negative, carry, and overflow flags
    /// as appropriate.
    ///
    /// The overflow flag is set if a signed addition would result in an overflow of the signed
    /// value.
    ///
    fn op_adc(&mut self, value: u8) {
        if self.p & Self::SR_BCD == 0 {
            let (mut result, mut carry) = self.ac.overflowing_add(value);
            if (self.p & Self::SR_CARRY) != 0 {
                if result == 0xFF {
                    result = 0;
                    carry = true;
                } else {
                    result += 1;
                }
            }
            let overflow = ((self.ac ^ result) & (value ^ result) & 0x80) != 0;
            self.ac = result;
            self.set_overflow(overflow);
            self.set_carry(carry);
            self.set_nz(self.ac);
        } else {
            let d1 = bcd_add_digits!(self.ac & 0x0F, value & 0x0F, self.p & Self::SR_CARRY);
            let d2 = bcd_add_digits!((self.ac >> 4), (value >> 4), d1 >> 4);
            self.ac = (d1 & 0x0F) | (d2 << 4);
            self.set_carry((d2 & 0x10) != 0);
        }
    }

    /// Adds the value to the accumulator, setting the zero, negative, carry, and overflow flags
    /// as appropriate.
    ///
    /// The overflow flag is set if a signed addition would result in an overflow of the signed
    /// value.
    ///
    fn op_sbc(&mut self, value: u8) {
        if self.p & Self::SR_BCD == 0 {
            let (mut result, mut borrow) = self.ac.overflowing_sub(value);
            if (self.p & Self::SR_CARRY) == 0 {
                if result == 0x00 {
                    result = 0xFF;
                    borrow = true;
                } else {
                    result -= 1;
                }
            }
            let overflow = ((self.ac ^ result) & ((255 - value) ^ result) & 0x80) != 0;
            self.ac = result;
            self.set_overflow(overflow);
            self.set_carry(!borrow);
            self.set_nz(self.ac);
        } else {
            let borrow = if (self.p & Self::SR_CARRY) == 0 { 1 } else { 0 };
            let d1 = bcd_add_digits!(self.ac & 0x0F, 10 - ((value & 0x0F) + borrow), 0);
            let d2 = bcd_add_digits!((self.ac >> 4), 10 - ((value >> 4) + (1 - (d1 >> 4))), 0);
            self.ac = (d1 & 0x0F) | (d2 << 4);
            self.set_carry((d2 & 0x10) != 0);
        }
    }

    /// Compares the value with the accumulator, and sets flags as appropriate.
    ///
    fn op_cmp(&mut self, value: u8) {
        self.op_compare(value, self.ac);
    }

    /// Compares the value with the X register, and sets flags as appropriate.
    ///
    fn op_cpx(&mut self, value: u8) {
        self.op_compare(value, self.x);
    }

    /// Compares the value with the X register, and sets flags as appropriate.
    ///
    fn op_cpy(&mut self, value: u8) {
        self.op_compare(value, self.y);
    }

    /// Compares the value with another, and sets flags as appropriate.
    /// This is a helper function for the various comparison operations.
    ///
    fn op_compare(&mut self, value: u8, compare_to: u8) {
        let (result, carry) = compare_to.overflowing_sub(value);
        self.set_carry(!carry);
        self.set_nz(result);
    }

    /// Returns the value in the accumulator, for storage.
    ///
    fn op_sta(&mut self) -> u8 {
        self.ac
    }

    /// Returns the value in the X register, for storage.
    ///
    fn op_stx(&mut self) -> u8 {
        self.x
    }

    /// Returns the value in the Y register, for storage.
    ///
    fn op_sty(&mut self) -> u8 {
        self.y
    }

    /// Sets the zero and negative flags based on the operand.
    ///
    #[inline(always)]
    fn set_nz(&mut self, value: u8) {
        self.p = self.p & !(Self::SR_ZERO | Self::SR_NEGATIVE)
            | (if value == 0 { Self::SR_ZERO } else { 0 })
            | (if value & 0x80 != 0 { Self::SR_NEGATIVE } else { 0 });
    }

    /// Sets or clears the carry flag.
    ///
    #[inline(always)]
    fn set_carry(&mut self, value: bool) {
        self.p = if value {
            self.p | Self::SR_CARRY
        } else {
            self.p & !Self::SR_CARRY
        };
    }

    /// Sets or clears the overflow flag.
    ///
    #[inline(always)]
    fn set_overflow(&mut self, value: bool) {
        self.p = if value {
            self.p | Self::SR_OVERFLOW
        } else {
            self.p & !Self::SR_OVERFLOW
        };
    }
}

impl Component for C6502 {
    fn run(&mut self, stop: Arc<AtomicBool>) {
        let mut cycles = 0;
        let mut start = Instant::now();
        loop {
            if cycles == 0 {
                start = Instant::now();
            }
            let signal = self.phi0_in.wait();
            if stop.load(Ordering::Relaxed) {
                break;
            }

            self.phi1_out.update(!signal);
            self.phi2_out.update(signal);
            if signal {
                self.step();
                cycles += 1;
            } else {
            }

            // TODO: Handle interrupts before next clock cycle
        }
        let elapsed = start.elapsed();
        println!(
            "Executed {} cycles in {} ms, speed {} MHz",
            cycles,
            elapsed.as_millis(),
            cycles as f64 / elapsed.as_millis() as f64 / 1000.0
        );
    }
}

enum Op {
    Read(fn(&mut C6502, u8)),
    ReadWrite(fn(&mut C6502, u8) -> u8),
    Write(fn(&mut C6502) -> u8),
    Implied(fn(&mut C6502)),
}

impl Op {
    fn is_read_or_implied(&self) -> bool {
        matches!(self, Op::Read(_) | Op::Implied(_))
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CpuState {
    Off,
    Resetting,
    Running,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CpuAction {
    Continue,
    Complete,
    CompleteAndFetch,
}

#[cfg(test)]
#[path = "./c6502_tests.rs"]
mod tests;
