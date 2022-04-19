use std::io::stdin;

use rustycoat::core::clock::*;
use rustycoat::core::memory::*;
use rustycoat::core::*;
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
    let mut rom_bytes: [u8; 0x2000] = [0; 0x2000];
    rom_bytes[0..RESET_PROGRAM.len()].copy_from_slice(RESET_PROGRAM);
    rom_bytes[0x1000..0x1000 + NMI_PROGRAM.len()].copy_from_slice(NMI_PROGRAM);
    rom_bytes[0x1100..0x1100 + IRQ_PROGRAM.len()].copy_from_slice(IRQ_PROGRAM);
    rom_bytes[0x1ffa..].copy_from_slice(&[0x00, 0xf0, 0x00, 0xe0, 0x00, 0xf1]);

    // Create a new memory object with a ROM loaded into the top 8K
    let memory = Memory::new();
    memory.configure_banks(vec![RomBank::with_bytes(&rom_bytes)], &[(0xe000, 0x2000, 1, 0x0000)]);

    // Create a CPU instance wired to the memory.
    let mut cpu = C6502::new(&memory);
    cpu.reset();

    // Create a 1MHz clock and wire it up to the CPU.
    let mut clock = Clock::new(1_000_000);
    clock.output().connect_to(cpu.clock_in());

    // Create a computer, add components, and start it up.
    let mut c = Computer::new();
    c.add(cpu);
    c.add(clock);
    c.start();

    println!("Hit enter to stop");
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    c.stop();
}
