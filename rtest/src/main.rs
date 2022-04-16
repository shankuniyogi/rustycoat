use std::time;

use rustycoat::core::memory::*;
use rustycoat::cpus::c6502::*;

const RESET_PROGRAM: &[u8] = &[
    0xA9, 0x00, // LDA #$00
    0x85, 0x05, // STA $05
    0xA5, 0x05, // LDA $05
    0x69, 0x01, // ADC #$01
    0x85, 0x05, // STA $05
    0x4C, 0x04, 0xE0, // JMP $E004
];

const NMI_PROGRAM: &[u8] = &[];
const IRQ_PROGRAM: &[u8] = &[];

fn main() {
    let memory = Memory::new_shared();

    let mut rom_bytes: [u8; 0x2000] = [0; 0x2000];
    rom_bytes[0..RESET_PROGRAM.len()].copy_from_slice(RESET_PROGRAM);
    rom_bytes[0x1000..0x1000 + NMI_PROGRAM.len()].copy_from_slice(NMI_PROGRAM);
    rom_bytes[0x1100..0x1100 + IRQ_PROGRAM.len()].copy_from_slice(IRQ_PROGRAM);
    rom_bytes[0x1ffa..].copy_from_slice(&[0x00, 0xf0, 0x00, 0xe0, 0x00, 0xf1]);
    memory
        .borrow_mut()
        .configure_banks(vec![RomBank::with_bytes(&rom_bytes)], &[(0xe000, 0x2000, 1, 0x0000)]);

    let cpu = C6502::new_shared(&memory);
    cpu.borrow_mut().reset();

    // Run 1,000,000 cycles
    let start = time::Instant::now();
    let mut cpu_borrowed = cpu.borrow_mut();
    for _ in 0..1_000_000 {
        cpu_borrowed.step();
    }
    let duration = start.elapsed();
    println!(
        "Time to run 1M cycles: {:?}. Effective clock speed: {} MHz",
        duration,
        1.0 / (duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0)
    );
}
