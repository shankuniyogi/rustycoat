# rustycoat

Rustycoat is a fun little project to create a 6502 emulator in
Rust. 

The [6502](https://en.wikipedia.org/wiki/MOS_Technology_6502) was a 
revolutionary 8-bit microprocessor - its simple and inexpensive design
made it practical to build low-cost personal computers, and together with
the [Z80](https://en.wikipedia.org/wiki/Zilog_Z80), the 6502 really
kickstarted the home computer movement. The 6502, or variants of it,
were used in Apple, Atari, Commodore and Nintendo machines.   

The core CPU emulator (found in [c6502.rs](src/cpus/c6502.rs))
is pretty much complete, and timing-accurate with the 6502 chip. Tests
of the instruction set are found in [c6502_tests.rs](src/cpus/c6502_tests.rs).

Along with the CPU, there are software equivalents of a few components:
a [clock](src/core/clock.rs) with adjustable frequency, a [memory](src/core/memory.rs)
implementation with customizable banks (compatible with the 6502's bank-switching
memory architecture), a set of [gates](src/gates/mod.rs), and even some "UI", in the
form of [LEDs](src/widgets/leds.rs) built on top of the [iui](https://github.com/rust-native-ui/libui-rs) crate.

The programming model is designed to make it easy to hook up and run the CPU with a clock,
memory, and some set of components - much like one would with a breadboard. Here's a trivial
example that wires up a clock directly to an LED:

```
    // Create an LED
    let mut led = Led::new(Color::new(1.0, 0.0, 0.0), Color::new(0.4, 0.4, 0.4));

    // Create a 5Hz clock and wire it up to the LED.
    let mut clock = Clock::new(5);
    clock.output().connect_to(led.input());

    // Create the computer, add components, and start it up.
    let mut c = Computer::new();
    c.add_async(clock);
    c.add_ui(led);

    c.run();
```

An actual CPU example can be found in the [examples](examples) directory.

This was mostly just a fun project to go down memory lane and learn a bit
of Rust while doing it. At some point, maybe it could be grown into an 
actual computer emulator.

# References

While working on Rustycoat, I came across a lot of great materials on the 6502.

- A good [summary](https://retrocomputing.stackexchange.com/questions/17888/what-is-the-mos-6502-doing-on-each-cycle-of-an-instruction) of what the 6502 does on each cycle of an instruction.

- The best [cycle-by-cycle guide](http://www.atarihq.com/danb/files/64doc.txt) of a 6502's instruction set.

- 6502 instruction set with [length and timings](http://6502.org/tutorials/6502opcodes.html) of each instruction.

- Another instruction set [guide](https://www.masswerk.at/6502/6502_instruction_set.html), with better formatting.

- A great description of the 6502's [overflow flag](http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html).

- A description of how [interrupts](http://6502.org/tutorials/interrupts.html) work on a 6502.

- The original 6502 [datasheet](http://archive.6502.org/datasheets/mos_6510_mpu.pdf), helpful to understand the hardware and pin layouts.

- A fun list of [bugs](https://www.liquisearch.com/mos_technology_6502/bugs_and_quirks) on the 6502 and its variants.

- Another list of 6502 [bugs and implementation quirks](https://atariwiki.org/wiki/Wiki.jsp?page=6502%20bugs).

- An explanation of when the [status register](https://stackoverflow.com/questions/47532801/when-is-the-status-register-updated-in-the-6502) on a 6502 is updated.

- A great guide on how [comparison instructions](http://6502.org/tutorials/compare_instructions.html) on a 6502 work.

- A guide to the C64 [memory layout](https://www.pagetable.com/c64ref/c64mem/).

- A guide to how the C64 [keyboard](http://c64os.com/post/howthekeyboardworks) works.