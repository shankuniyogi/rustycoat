use super::*;
use std::cell::RefCell;
use std::rc::Rc;

struct CpuTest {
    mem: Rc<RefCell<Memory>>,
    cpu: Rc<RefCell<C6502>>,
    ins_location: u16,
    ac: u8,
    x: u8,
    y: u8,
    sp: u8,
    p: u8,
    pc: u16,
    cycles: usize,
}

impl CpuTest {
    fn new() -> Self {
        let mem = Memory::new_shared();
        let cpu = C6502::new_shared(&mem);
        CpuTest {
            mem,
            cpu,
            ins_location: 0x0400,
            pc: 0x0400,
            ac: 0,
            x: 0,
            y: 0,
            sp: 0xFF,
            p: 0,
            cycles: 0,
        }
    }

    fn with_pc(&mut self, pc: u16) -> &mut Self {
        self.ins_location = pc;
        self.pc = pc;
        self
    }

    fn with_instruction(&mut self, ins_bytes: &[u8]) -> &mut Self {
        self.mem.borrow_mut().write_block(self.ins_location, ins_bytes);
        self.ins_location += ins_bytes.len() as u16;
        self
    }

    fn with_data(&mut self, location: u16, data: &[u8]) -> &mut Self {
        self.mem.borrow_mut().write_block(location, data);
        self
    }

    fn with_state(&mut self, init_fn: fn(&mut Self)) -> &mut Self {
        init_fn(self);
        self
    }

    fn with_stack(&mut self, stack: &[u8]) -> &mut Self {
        self.sp = 0xFF - stack.len() as u8;
        self.mem.borrow_mut().write_block(C6502::STACK_BASE + self.sp as u16 + 1, stack);
        self
    }

    fn run_one(&mut self) -> &mut Self {
        self.run(1)
    }

    fn run(&mut self, instruction_count: usize) -> &mut Self {
        let mut cpu = self.cpu.borrow_mut();
        cpu.pc = self.pc;
        cpu.ac = self.ac;
        cpu.x = self.x;
        cpu.y = self.y;
        cpu.sp = self.sp;
        cpu.p = self.p;
        cpu.state = CpuState::Running;

        let mut last_action = CpuAction::Continue;
        for _ in 0..instruction_count {
            loop {
                self.cycles += 1;
                last_action = cpu.step();
                if last_action != CpuAction::Continue {
                    break;
                }
            }
        }
        if last_action == CpuAction::CompleteAndFetch {
            self.cycles -= 1;
        }

        self.pc = cpu.pc;
        self.ac = cpu.ac;
        self.x = cpu.x;
        self.y = cpu.y;
        self.sp = cpu.sp;
        self.p = cpu.p;

        drop(cpu);
        self
    }

    fn data(&self, location: u16) -> u8 {
        self.mem.borrow().read_byte(location)
    }

    fn stack(&self, pos: u8) -> u8 {
        self.mem.borrow().read_byte(C6502::STACK_BASE + self.sp as u16 + 1 + pos as u16)
    }

    fn values<T>(&self, observe_fn: fn(&Self) -> T) -> T {
        observe_fn(self)
    }
}

#[test]
fn cpu_addressing_modes_read() {
    // Immediate - LDA #48
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xA9, 0x48])
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 2)
    );

    // Zero Page - LDA $50
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xA5, 0x50])
            .with_data(0x50, &[0x48])
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 3)
    );

    // Zero-page, X-indexed - LDA $40,X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB5, 0x40])
            .with_data(0x50, &[0x48])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 4)
    );

    // Zero-page X-indexed with page wrapping
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x15, 0x80])
            .with_data(0x10, &[0x48])
            .with_state(|c| c.x = 0x90)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 4)
    );

    // Zero-page, Y-indexed - LDX $40,Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB6, 0x40])
            .with_data(0x50, &[0x48])
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.x, c.cycles)),
        (0x48, 4)
    );

    // Zero-page, Y-indexed with page wrapping
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB6, 0x80])
            .with_data(0x10, &[0x48])
            .with_state(|c| c.y = 0x90)
            .run_one()
            .values(|c| (c.x, c.cycles)),
        (0x48, 4)
    );

    // Absolute - LDA $1000
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xAD, 0x00, 0x10])
            .with_data(0x1000, &[0x48])
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 4)
    );

    // Absolute, Y-indexed - LDA $1000,Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB9, 0x00, 0x10])
            .with_data(0x1040, &[0x48])
            .with_state(|c| c.y = 0x40)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 4)
    );

    // Absolute, Y-indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB9, 0x80, 0x1F])
            .with_data(0x2000, &[0x48])
            .with_state(|c| c.y = 0x80)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 5)
    );

    // Absolute, X-indexed - LDA $1000,X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xBD, 0x00, 0x10])
            .with_data(0x1040, &[0x48])
            .with_state(|c| c.x = 0x40)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 4)
    );

    // Absolute, X-indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xBD, 0x80, 0x1F])
            .with_data(0x2000, &[0x48])
            .with_state(|c| c.x = 0x80)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 5)
    );

    // Indexed indirect - LDA ($40,X)
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xA1, 0x40])
            .with_data(0x80, &[0x00, 0x10])
            .with_data(0x1000, &[0x48])
            .with_state(|c| c.x = 0x40)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 6)
    );

    // Indexed indirect with wrapping (an x-index that goes past
    // the end of the zero page wraps back to the start of the zero page)
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xA1, 0x80])
            .with_data(0x10, &[0x00, 0x10])
            .with_data(0x1000, &[0x48])
            .with_state(|c| c.x = 0x90)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 6)
    );

    // Indirect indexed - LDA ($80),Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB1, 0x80])
            .with_data(0x80, &[0x00, 0x10])
            .with_data(0x1040, &[0x48])
            .with_state(|c| c.y = 0x40)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 5)
    );

    // Indirect indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB1, 0x80])
            .with_data(0x80, &[0xF0, 0x1F])
            .with_data(0x2000, &[0x48])
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 6)
    );
}

#[test]
fn cpu_addressing_modes_write() {
    // Zero-page - STA $50
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x85, 0x50])
            .with_state(|c| c.ac = 0x48)
            .run_one()
            .values(|c| (c.data(0x50), c.cycles)),
        (0x48, 3)
    );

    // Zero-page, X-indexed - STA $40,X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x95, 0x40])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.data(0x50), c.cycles)),
        (0x48, 4)
    );

    // Zero-page, X-indexed with page wrapping
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x95, 0x80])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.x = 0x90)
            .run_one()
            .values(|c| (c.data(0x10), c.cycles)),
        (0x48, 4)
    );

    // Zero-page, Y-indexed - STX $40,Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x96, 0x40])
            .with_state(|c| c.x = 0x48)
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.data(0x50), c.cycles)),
        (0x48, 4)
    );

    // Zero-page, Y-indexed with page wrapping
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x96, 0x80])
            .with_state(|c| c.x = 0x48)
            .with_state(|c| c.y = 0x90)
            .run_one()
            .values(|c| (c.data(0x10), c.cycles)),
        (0x48, 4)
    );

    // Absolute - STA $1000
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x8D, 0x00, 0x10])
            .with_state(|c| c.ac = 0x48)
            .run_one()
            .values(|c| (c.data(0x1000), c.cycles)),
        (0x48, 4)
    );

    // Absolute, Y-indexed - STA $1000,Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x99, 0x00, 0x10])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.y = 0x40)
            .run_one()
            .values(|c| (c.data(0x1040), c.cycles)),
        (0x48, 5)
    );

    // Absolute, Y-indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x99, 0xF0, 0x1F])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.data(0x2000), c.cycles)),
        (0x48, 5)
    );

    // Absolute, X-indexed - STA $1000,X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x9D, 0x00, 0x10])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.x = 0x40)
            .run_one()
            .values(|c| (c.data(0x1040), c.cycles)),
        (0x48, 5)
    );

    // Absolute, X-indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x9D, 0xF0, 0x1F])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.data(0x2000), c.cycles)),
        (0x48, 5)
    );

    // Indexed indirect - STA ($40,X)
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x81, 0x40])
            .with_data(0x80, &[0x00, 0x10])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.x = 0x40)
            .run_one()
            .values(|c| (c.data(0x1000), c.cycles)),
        (0x48, 6)
    );

    // Indexed indirect with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x81, 0x80])
            .with_data(0x10, &[0x00, 0x10])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.x = 0x90)
            .run_one()
            .values(|c| (c.data(0x1000), c.cycles)),
        (0x48, 6)
    );

    // Indirect indexed - STA ($80),Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x91, 0x80])
            .with_data(0x80, &[0x00, 0x10])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.y = 0x40)
            .run_one()
            .values(|c| (c.data(0x1040), c.cycles)),
        (0x48, 6)
    );

    // Indirect indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x91, 0x80])
            .with_data(0x80, &[0xF0, 0x1F])
            .with_state(|c| c.ac = 0x48)
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.data(0x2000), c.cycles)),
        (0x48, 6)
    );
}

#[test]
fn cpu_addressing_modes_read_write() {
    // Accumulator - ASL A
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0A])
            .with_state(|c| c.ac = 0x24)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 2)
    );

    // Zero-page - ASL $50
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x06, 0x50])
            .with_data(0x50, &[0x24])
            .run_one()
            .values(|c| (c.data(0x50), c.cycles)),
        (0x48, 5)
    );

    // Zero-page, X-indexed - ASL $40,X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x16, 0x40])
            .with_data(0x50, &[0x24])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.data(0x50), c.cycles)),
        (0x48, 6)
    );

    // Zero-page, X-indexed with zero-page wrapping
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x16, 0x80])
            .with_data(0x10, &[0x24])
            .with_state(|c| c.x = 0x90)
            .run_one()
            .values(|c| (c.data(0x10), c.cycles)),
        (0x48, 6)
    );

    // Absolute - ASL $1040
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0E, 0x40, 0x10])
            .with_data(0x1040, &[0x24])
            .run_one()
            .values(|c| (c.data(0x1040), c.cycles)),
        (0x48, 6)
    );

    // Absolute, X-indexed - ASL $1000,X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x1E, 0x00, 0x10])
            .with_data(0x1040, &[0x24])
            .with_state(|c| c.x = 0x40)
            .run_one()
            .values(|c| (c.data(0x1040), c.cycles)),
        (0x48, 7)
    );

    // Absolute, X-indexed with page crossing
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x1E, 0xF0, 0x1F])
            .with_data(0x2000, &[0x24])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.data(0x2000), c.cycles)),
        (0x48, 7)
    );
}

#[test]
fn test_branch_cycle_counts() {
    // Branch forward to relative address on same page
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xF0, 0x0E])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0410, 3)
    );

    // Branch backward to relative address on same page
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xF0, 0xFF])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0401, 3)
    );

    // Branch backward to relative address on previous page
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xF0, 0xF0])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x03F2, 4)
    );

    // Branch forward to relative address on next page
    assert_eq_hex!(
        CpuTest::new()
            .with_pc(0x04F0)
            .with_instruction(&[0xF0, 0x10])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0502, 4)
    );

    // Failed branch
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xF0, 0x04])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0402, 2)
    );
}

#[test]
fn test_adc() {
    // The non-BCD test cases are courtesy of the excellent post on the
    // overflow flag at http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html.

    // Add two numbers with no unsigned carry-out or signed overflow
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x10])
            .with_state(|c| c.ac = 0x50)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x60, 0x00)
    );

    // Add two numbers with signed overflow but no unsigned carry-out,
    // and a negative signed result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x50])
            .with_state(|c| c.ac = 0x50)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xA0, C6502::SR_OVERFLOW | C6502::SR_NEGATIVE)
    );

    // Add two numbers with no unsigned carry-out or signed overflow,
    // but a negative signed result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x90])
            .with_state(|c| c.ac = 0x50)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xE0, C6502::SR_NEGATIVE)
    );

    // Add two numbers with unsigned carry-out, but no signed overflow
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0xD0])
            .with_state(|c| c.ac = 0x50)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, C6502::SR_CARRY)
    );

    // Add two numbers with no unsigned carry-out or signed overflow,
    // but a negative signed result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x10])
            .with_state(|c| c.ac = 0xD0)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xE0, C6502::SR_NEGATIVE)
    );

    // Add two numbers with unsigned carry-out, but no signed overflow.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x50])
            .with_state(|c| c.ac = 0xD0)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, C6502::SR_CARRY)
    );

    // Add two numbers with unsigned carry-out and signed overflow.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x90])
            .with_state(|c| c.ac = 0xD0)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x60, C6502::SR_CARRY | C6502::SR_OVERFLOW)
    );

    // Add two numbers with unsigned carry-out but no signed overflow,
    // and a negative signed result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0xD0])
            .with_state(|c| c.ac = 0xD0)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xA0, C6502::SR_CARRY | C6502::SR_NEGATIVE)
    );

    // Verify that carry-in works.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x10])
            .with_state(|c| c.ac = 0x40)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x51, 0x00)
    );

    // Verify that zero flag is set when result is zero.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x00])
            .with_state(|c| c.ac = 0x00)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Add two numbers in BCD mode without carry
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x28])
            .with_state(|c| c.ac = 0x22)
            .with_state(|c| c.p = C6502::SR_BCD)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x50, C6502::SR_BCD)
    );

    // Add two numbers in BCD mode with carry-in
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x28])
            .with_state(|c| c.ac = 0x22)
            .with_state(|c| c.p = C6502::SR_BCD | C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x51, C6502::SR_BCD)
    );

    // Add two numbers in BCD mode with carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x69, 0x29])
            .with_state(|c| c.ac = 0x72)
            .with_state(|c| c.p = C6502::SR_BCD)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x01, C6502::SR_BCD | C6502::SR_CARRY)
    );
}

#[test]
fn test_and() {
    // And #$24 and #$28 to get #$20
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x29, 0x28])
            .with_state(|c| c.ac = 0x24)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, 0x00)
    );

    // And #$84 and #$82 to get #$80
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x29, 0x82])
            .with_state(|c| c.ac = 0x84)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );

    // And #$40 and #$04 to get #$00
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x29, 0x40])
            .with_state(|c| c.ac = 0x04)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );
}

#[test]
fn test_asl() {
    // Shift left a number with no carry
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0A])
            .with_state(|c| c.ac = 0x24)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x48, 0x00)
    );

    // Shift left zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0A])
            .with_state(|c| c.ac = 0x00)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Shift left to get a negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0A])
            .with_state(|c| c.ac = 0x41)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x82, C6502::SR_NEGATIVE)
    );

    // Shift left to get a carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0A])
            .with_state(|c| c.ac = 0x84)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x08, C6502::SR_CARRY)
    );
}

#[test]
fn test_bcc() {
    // Branch if carry is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x90, 0x10]).run_one().values(|c| c.pc),
        0x0412
    );

    // Don't branch if carry is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x90, 0x10])
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_bcs() {
    // Branch if carry is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB0, 0x10])
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| c.pc),
        0x0412
    );

    // Don't branch if carry is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xB0, 0x10]).run_one().values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_beq() {
    // Branch if zero is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xF0, 0x10])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| c.pc),
        0x0412
    );

    // Don't branch if zero is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xF0, 0x10]).run_one().values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_bit() {
    // Bit test resulting in zero flag set.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x24, 0x10])
            .with_data(0x10, &[0x28])
            .with_state(|c| c.ac = 0x10)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x10, C6502::SR_ZERO)
    );

    // Bit test resulting in zero flag clear.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x24, 0x10])
            .with_data(0x10, &[0x28])
            .with_state(|c| c.ac = 0x20)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, 0x00)
    );

    // Bit test resulting in negative flag set.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x24, 0x10])
            .with_data(0x10, &[0x88])
            .with_state(|c| c.ac = 0x08)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x08, C6502::SR_NEGATIVE)
    );

    // Bit test resulting in overflow flag set.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x24, 0x10])
            .with_data(0x10, &[0x48])
            .with_state(|c| c.ac = 0x08)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x08, C6502::SR_OVERFLOW)
    );
}

#[test]
fn test_bmi() {
    // Branch if negative is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x30, 0x10])
            .with_state(|c| c.p = C6502::SR_NEGATIVE)
            .run_one()
            .values(|c| c.pc),
        0x0412
    );

    // Don't branch if negative is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x30, 0x10]).run_one().values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_bne() {
    // Branch if zero is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xD0, 0x10]).run_one().values(|c| c.pc),
        0x0412
    );

    // Don't branch if zero is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xD0, 0x10])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_bpl() {
    // Branch if negative is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x10, 0x10]).run_one().values(|c| c.pc),
        0x0412
    );

    // Don't branch if negative is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x10, 0x10])
            .with_state(|c| c.p = C6502::SR_NEGATIVE)
            .run_one()
            .values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_brk() {
    // Test a BRK
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x00])
            .with_data(0xFFFE, &[0x48, 0x84])
            .with_state(|c| c.p = C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.pc, c.sp, c.stack(0), c.stack(1), c.stack(2), c.cycles)),
        (0x8448, 0xFC, C6502::SR_ZERO | C6502::SR_BREAK | C6502::SR_UNUSED, 0x02, 0x04, 7)
    );
}

#[test]
fn test_bvc() {
    // Branch if overflow is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x50, 0x10]).run_one().values(|c| c.pc),
        0x0412
    );

    // Don't branch if overflow is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x30, 0x10])
            .with_state(|c| c.p = C6502::SR_OVERFLOW)
            .run_one()
            .values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_bvs() {
    // Branch if overflow is set
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x70, 0x10])
            .with_state(|c| c.p = C6502::SR_OVERFLOW)
            .run_one()
            .values(|c| c.pc),
        0x0412
    );

    // Don't branch if overflow is clear
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x70, 0x10]).run_one().values(|c| c.pc),
        0x0402
    );
}

#[test]
fn test_clc() {
    // Clear carry flag
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x18])
            .with_state(|c| c.p = C6502::SR_CARRY | C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.p, c.cycles)),
        (C6502::SR_ZERO, 2)
    );
}

#[test]
fn test_cld() {
    // Clear decimal flag
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xD8])
            .with_state(|c| c.p = C6502::SR_BCD | C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.p, c.cycles)),
        (C6502::SR_ZERO, 2)
    );
}

#[test]
fn test_cli() {
    // Clear interrupt flag
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x58])
            .with_state(|c| c.p = C6502::SR_INTERRUPT_MASK | C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.p, c.cycles)),
        (C6502::SR_ZERO, 2)
    );
}

#[test]
fn test_clv() {
    // Clear overflow flag
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xB8])
            .with_state(|c| c.p = C6502::SR_OVERFLOW | C6502::SR_ZERO)
            .run_one()
            .values(|c| (c.p, c.cycles)),
        (C6502::SR_ZERO, 2)
    );
}

#[test]
fn test_cmp() {
    // Compare A < M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC9, 0x40])
            .with_state(|c| c.ac = 0x10)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x10, C6502::SR_NEGATIVE)
    );

    // Compare A == M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC9, 0x10])
            .with_state(|c| c.ac = 0x10)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x10, C6502::SR_ZERO | C6502::SR_CARRY)
    );

    // Compare A > M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC9, 0x10])
            .with_state(|c| c.ac = 0x40)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x40, C6502::SR_CARRY)
    );
}

#[test]
fn test_cpx() {
    // Compare X < M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE0, 0x40])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x10, C6502::SR_NEGATIVE)
    );

    // Compare X == M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE0, 0x10])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x10, C6502::SR_ZERO | C6502::SR_CARRY)
    );

    // Compare X > M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE0, 0x10])
            .with_state(|c| c.x = 0x40)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x40, C6502::SR_CARRY)
    );
}

#[test]
fn test_cpy() {
    // Compare Y < M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC0, 0x40])
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x10, C6502::SR_NEGATIVE)
    );

    // Compare Y == M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC0, 0x10])
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x10, C6502::SR_ZERO | C6502::SR_CARRY)
    );

    // Compare Y > M
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC0, 0x10])
            .with_state(|c| c.y = 0x40)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x40, C6502::SR_CARRY)
    );
}

#[test]
fn test_dec() {
    // Decrement number to non-negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC6, 0x10])
            .with_data(0x0010, &[0x10])
            .run_one()
            .values(|c| (c.data(0x0010), c.p)),
        (0x0F, 0x00)
    );

    // Decrement number to zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC6, 0x10])
            .with_data(0x0010, &[0x01])
            .run_one()
            .values(|c| (c.data(0x0010), c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Decrement number to negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC6, 0x10])
            .with_data(0x0010, &[0x00])
            .run_one()
            .values(|c| (c.data(0x0010), c.p)),
        (0xFF, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_dex() {
    // Decrement X to non-negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xCA])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x0F, 0x00)
    );

    // Decrement X to zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xCA])
            .with_state(|c| c.x = 0x01)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Decrement X to negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xCA])
            .with_state(|c| c.x = 0x00)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0xFF, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_dey() {
    // Decrement Y to non-negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x88])
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x0F, 0x00)
    );

    // Decrement Y to zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x88])
            .with_state(|c| c.y = 0x01)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Decrement Y to negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x88])
            .with_state(|c| c.y = 0x00)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0xFF, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_eor() {
    // XORr #$28 and #$48 to get #$60
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x49, 0x48])
            .with_state(|c| c.ac = 0x28)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x60, 0x00)
    );

    // XOR #$28 and #$88 to get #$A0
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x49, 0x88])
            .with_state(|c| c.ac = 0x28)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xA0, C6502::SR_NEGATIVE)
    );

    // XOR #$40 and #$40 to get #$00
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x49, 0x40])
            .with_state(|c| c.ac = 0x40)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );
}

#[test]
fn test_inc() {
    // Increment number to non-negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE6, 0x10])
            .with_data(0x0010, &[0x10])
            .run_one()
            .values(|c| (c.data(0x0010), c.p)),
        (0x11, 0x00)
    );

    // Increment number to zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE6, 0x10])
            .with_data(0x0010, &[0xFF])
            .with_state(|c| c.p = C6502::SR_NEGATIVE)
            .run_one()
            .values(|c| (c.data(0x0010), c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Increment number to negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE6, 0x10])
            .with_data(0x0010, &[0x7F])
            .run_one()
            .values(|c| (c.data(0x0010), c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_inx() {
    // Increment X to non-negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE8])
            .with_state(|c| c.x = 0x10)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x11, 0x00)
    );

    // Increment X to zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE8])
            .with_state(|c| c.x = 0xFF)
            .with_state(|c| c.p = C6502::SR_NEGATIVE)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Increment X to negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE8])
            .with_state(|c| c.x = 0x7F)
            .run_one()
            .values(|c| (c.x, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_iny() {
    // Increment Y to non-negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC8])
            .with_state(|c| c.y = 0x10)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x11, 0x00)
    );

    // Increment Y to zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC8])
            .with_state(|c| c.y = 0xFF)
            .with_state(|c| c.p = C6502::SR_NEGATIVE)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Increment Y to negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xC8])
            .with_state(|c| c.y = 0x7F)
            .run_one()
            .values(|c| (c.y, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_jmp() {
    // Test absolute jump
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x4C, 0x48, 0x20])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x2048, 3)
    );

    // Test indirect jump
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6C, 0x00, 0x10])
            .with_data(0x1000, &[0x48, 0x20])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x2048, 5)
    );

    // Test indirect jump bug (crossing page boundary)
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6C, 0xFF, 0x1F])
            .with_data(0x1FFF, &[0x48])
            .with_data(0x1F00, &[0x20])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x2048, 5)
    );
}

#[test]
fn test_jsr() {
    // Test JSR
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x20, 0x48, 0x20]).run_one().values(|c| (
            c.pc,
            c.sp,
            c.stack(0),
            c.stack(1),
            c.cycles
        )),
        (0x2048, 0xFD, 0x02, 0x04, 6)
    );
}

#[test]
fn test_lda() {
    // Load non-zero number
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA9, 0x10]).run_one().values(|c| (c.ac, c.p)),
        (0x10, 0x00)
    );

    // Load zero
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA9, 0x00]).run_one().values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Load negative number
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA9, 0x80]).run_one().values(|c| (c.ac, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_ldx() {
    // Load non-zero number
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA2, 0x10]).run_one().values(|c| (c.x, c.p)),
        (0x10, 0x00)
    );

    // Load zero
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA2, 0x00]).run_one().values(|c| (c.x, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Load negative number
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA2, 0x80]).run_one().values(|c| (c.x, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_ldy() {
    // Load non-zero number
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA0, 0x10]).run_one().values(|c| (c.y, c.p)),
        (0x10, 0x00)
    );

    // Load zero
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA0, 0x00]).run_one().values(|c| (c.y, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Load negative number
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xA0, 0x80]).run_one().values(|c| (c.y, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );
}

#[test]
fn test_lsr() {
    // Shift right a number with no carry
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x4A])
            .with_state(|c| c.ac = 0x08)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x04, 0x00)
    );

    // Shift right a zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x4A])
            .with_state(|c| c.ac = 0x00)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Shift right to clear a negative
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x4A])
            .with_state(|c| c.ac = 0x80)
            .with_state(|c| c.p = C6502::SR_NEGATIVE)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x40, 0x00)
    );

    // Shift right a number to get a carry
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x4A])
            .with_state(|c| c.ac = 0x41)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, C6502::SR_CARRY)
    );
}

#[test]
fn test_nop() {
    // Implied mode
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xEA]).run_one().values(|c| (c.pc, c.cycles)),
        (0x0402, 2)
    );

    // Immediate mode
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x80, 0x10])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0403, 2)
    );

    // Zero page mode
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x04, 0x10])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0403, 3)
    );

    // Zero page, X mode
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x14, 0x10])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0403, 4)
    );

    // Absolute mode
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x0C, 0x00, 0x10])
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0404, 4)
    );

    // Absolute, X indexed
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x1C, 0x00, 0x10])
            .with_state(|c| c.x = 0x01)
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0404, 4)
    );

    // Absolute, X indexed crossing page boundary
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x1C, 0xFF, 0x10])
            .with_state(|c| c.x = 0x01)
            .run_one()
            .values(|c| (c.pc, c.cycles)),
        (0x0404, 5)
    );
}

#[test]
fn test_ora() {
    // Or #$24 and #$48 to get #$6C
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x09, 0x48])
            .with_state(|c| c.ac = 0x24)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x6C, 0x00)
    );

    // Or #$24 and #$80 to get #$A4
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x09, 0x80])
            .with_state(|c| c.ac = 0x24)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xA4, C6502::SR_NEGATIVE)
    );

    // Or #$00 and #$00 to get #$00
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x09, 0x00])
            .with_state(|c| c.ac = 0x00)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );
}

#[test]
fn test_pha() {
    // Push accumulator on stack
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x48])
            .with_state(|c| c.ac = 0x24)
            .run_one()
            .values(|c| (c.sp, c.stack(0), c.cycles)),
        (0xFE, 0x24, 3)
    );
}

#[test]
fn test_php() {
    // Push processor status on stack
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x08])
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.sp, c.stack(0), c.cycles)),
        (0xFE, C6502::SR_CARRY | C6502::SR_BREAK | C6502::SR_UNUSED, 3)
    );
}

#[test]
fn test_pla() {
    // Pull value from stack
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x68])
            .with_stack(&[0x24])
            .run_one()
            .values(|c| (c.ac, c.p, c.sp, c.cycles)),
        (0x24, 0x00, 0xFF, 4)
    );

    // Pull zero value from stack
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x68])
            .with_stack(&[0x00])
            .run_one()
            .values(|c| (c.ac, c.p, c.sp)),
        (0x00, C6502::SR_ZERO, 0xFF)
    );

    // Pull negative value from stack
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x68])
            .with_stack(&[0x80])
            .run_one()
            .values(|c| (c.ac, c.p, c.sp)),
        (0x80, C6502::SR_NEGATIVE, 0xFF)
    );
}

#[test]
fn test_plp() {
    // Pull processor status from stack
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x28])
            .with_stack(&[C6502::SR_CARRY | C6502::SR_BREAK | C6502::SR_UNUSED])
            .run_one()
            .values(|c| (c.p, c.sp, c.cycles)),
        (C6502::SR_CARRY, 0xFF, 4)
    );
}

#[test]
fn test_rol() {
    // Rotate left with no carry-in or carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x2A])
            .with_state(|c| c.ac = 0x08)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x10, 0x00)
    );

    // Rotate left zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x2A])
            .with_state(|c| c.ac = 0x00)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Rotate left to get negative number
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x2A])
            .with_state(|c| c.ac = 0x40)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x80, C6502::SR_NEGATIVE)
    );

    // Rotate left with carry-in, no carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x2A])
            .with_state(|c| c.ac = 0x08)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x11, 0x00)
    );

    // Rotate left with carry-in and carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x2A])
            .with_state(|c| c.ac = 0x88)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x11, C6502::SR_CARRY)
    );

    // Rotate left to get zero, and carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x2A])
            .with_state(|c| c.ac = 0x80)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_CARRY | C6502::SR_ZERO)
    );
}

#[test]
fn test_ror() {
    // Rotate right with no carry-in or carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6A])
            .with_state(|c| c.ac = 0x08)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x04, 0x00)
    );

    // Rotate right zero
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6A])
            .with_state(|c| c.ac = 0x00)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO)
    );

    // Rotate right to get zero and carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6A])
            .with_state(|c| c.ac = 0x01)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_CARRY | C6502::SR_ZERO)
    );

    // Rotate right with carry-in, no carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6A])
            .with_state(|c| c.ac = 0x08)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x84, C6502::SR_NEGATIVE)
    );

    // Rotate right with carry-in and carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x6A])
            .with_state(|c| c.ac = 0x09)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x84, C6502::SR_NEGATIVE | C6502::SR_CARRY)
    );
}

#[test]
fn test_rti() {
    // Return from interrupt
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x40])
            .with_stack(&[C6502::SR_CARRY | C6502::SR_BREAK | C6502::SR_UNUSED, 0x48, 0x20])
            .run_one()
            .values(|c| (c.pc, c.p, c.sp, c.cycles)),
        (0x2048, C6502::SR_CARRY, 0xFF, 6)
    );
}

#[test]
fn test_rts() {
    // Return from subroutine. PC will be incremented by 1 from what is on the stack.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x60])
            .with_stack(&[0x48, 0x20])
            .run_one()
            .values(|c| (c.pc, c.sp, c.cycles)),
        (0x2049, 0xFF, 6)
    );
}

#[test]
fn test_sbc() {
    // The non-BCD test cases are courtesy of the excellent post on the
    // overflow flag at http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html.
    // The 6502 uses the inverse of the carry flag as a borrow flag (set when
    // there is no borrow, cleared when there is a borrow).

    // Subtract with unsigned borrow but no signed overflow.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0xF0])
            .with_state(|c| c.ac = 0x50)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x60, 0x00)
    );

    // Subtract with unsigned borrow and signed overflow, and a
    // signed negative result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0xB0])
            .with_state(|c| c.ac = 0x50)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xA0, C6502::SR_OVERFLOW | C6502::SR_NEGATIVE)
    );

    // Subtract with unsigned borrow but no signed overflow, and a
    // signed negative result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x70])
            .with_state(|c| c.ac = 0x50)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xE0, C6502::SR_NEGATIVE)
    );

    // Subtract with no unsigned borrow or signed overflow.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x30])
            .with_state(|c| c.ac = 0x50)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, C6502::SR_CARRY)
    );

    // Subtract with unsigned borrow but no signed overflow,
    // and a signed negative result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0xF0])
            .with_state(|c| c.ac = 0xD0)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xE0, C6502::SR_NEGATIVE)
    );

    // Subtract with no unsigned borrow or signed overflow.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0xB0])
            .with_state(|c| c.ac = 0xD0)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x20, C6502::SR_CARRY)
    );

    // Subtract with no unsigned borrow but a signed overflow.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x70])
            .with_state(|c| c.ac = 0xD0)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x60, C6502::SR_CARRY | C6502::SR_OVERFLOW)
    );

    // Subtract with no unsigned borrow or signed overflow,
    // and a signed negative result.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x30])
            .with_state(|c| c.ac = 0xD0)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0xA0, C6502::SR_CARRY | C6502::SR_NEGATIVE)
    );

    // Verify that borrow-in works.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x20])
            .with_state(|c| c.ac = 0x40)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x1F, C6502::SR_CARRY)
    );

    // Verify that the zero flag is set when the result is zero.
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x20])
            .with_state(|c| c.ac = 0x20)
            .with_state(|c| c.p = C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x00, C6502::SR_ZERO | C6502::SR_CARRY)
    );

    // Subtract two numbers in BCD mode without carry
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x28])
            .with_state(|c| c.ac = 0x50)
            .with_state(|c| c.p = C6502::SR_BCD | C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x22, C6502::SR_BCD | C6502::SR_CARRY)
    );

    // Subtract two numbers in BCD mode with carry-in
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x28])
            .with_state(|c| c.ac = 0x50)
            .with_state(|c| c.p = C6502::SR_BCD)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x21, C6502::SR_BCD | C6502::SR_CARRY)
    );

    // Subtract two numbers in BCD mode with carry-out
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xE9, 0x29])
            .with_state(|c| c.ac = 0x28)
            .with_state(|c| c.p = C6502::SR_BCD | C6502::SR_CARRY)
            .run_one()
            .values(|c| (c.ac, c.p)),
        (0x99, C6502::SR_BCD)
    );
}

#[test]
fn test_sec() {
    // Set carry flag
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x38]).run_one().values(|c| (c.p, c.cycles)),
        (C6502::SR_CARRY, 2)
    );
}

#[test]
fn test_sed() {
    // Set decimal flag
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xF8]).run_one().values(|c| (c.p, c.cycles)),
        (C6502::SR_BCD, 2)
    );
}

#[test]
fn test_sei() {
    // Set interrupt disable flag
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0x78]).run_one().values(|c| (c.p, c.cycles)),
        (C6502::SR_INTERRUPT_MASK, 2)
    );
}

#[test]
fn test_sta() {
    // Store accumulator in memory
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x85, 0x20])
            .with_state(|c| c.ac = 0x48)
            .run_one()
            .values(|c| (c.data(0x20))),
        0x48
    );
}

#[test]
fn test_stx() {
    // Store X register in memory
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x86, 0x20])
            .with_state(|c| c.x = 0x48)
            .run_one()
            .values(|c| (c.data(0x20))),
        0x48
    );
}

#[test]
fn test_sty() {
    // Store Y register in memory
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x84, 0x20])
            .with_state(|c| c.y = 0x48)
            .run_one()
            .values(|c| (c.data(0x20))),
        0x48
    );
}

#[test]
fn test_tax() {
    // Transfer accumulator to X
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xAA])
            .with_state(|c| c.ac = 0x48)
            .run_one()
            .values(|c| (c.x, c.cycles)),
        (0x48, 2)
    );
}

#[test]
fn test_tay() {
    // Transfer accumulator to Y
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0xA8])
            .with_state(|c| c.ac = 0x48)
            .run_one()
            .values(|c| (c.y, c.cycles)),
        (0x48, 2)
    );
}

#[test]
fn test_tsx() {
    // Transfer stack pointer to X
    assert_eq_hex!(
        CpuTest::new().with_instruction(&[0xBA]).run_one().values(|c| (c.x, c.cycles)),
        (0xFF, 2)
    );
}

#[test]
fn test_txa() {
    // Transfer X to accumulator
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x8A])
            .with_state(|c| c.x = 0x48)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 2)
    );
}

#[test]
fn test_txs() {
    // Transfer X to stack pointer
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x9A])
            .with_state(|c| c.x = 0x48)
            .run_one()
            .values(|c| (c.sp, c.cycles)),
        (0x48, 2)
    );
}

#[test]
fn test_tya() {
    // Transfer Y to accumulator
    assert_eq_hex!(
        CpuTest::new()
            .with_instruction(&[0x98])
            .with_state(|c| c.y = 0x48)
            .run_one()
            .values(|c| (c.ac, c.cycles)),
        (0x48, 2)
    );
}
