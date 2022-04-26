use rustycoat::core::clock::*;
use rustycoat::core::*;
use rustycoat::widgets::*;
use rustycoat::widgets::leds::*;

fn main() {
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
}
