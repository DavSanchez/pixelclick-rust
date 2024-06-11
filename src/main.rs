//! Blinks an LED
//!
//! This assumes that a LED is connected to the pin assigned to `led`. (GPIO0)

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Io, Level, Output},
    peripherals::Peripherals,
    prelude::*,
    rmt::Rmt,
    system::SystemControl,
};
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Set GPIO0 as an output, and set its state high initially.
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    let rmt = Rmt::new(peripherals.RMT, 80.MHz(), &clocks, None).unwrap();
    let rmt_buffer = smartLedBuffer!(1);
    let mut a_led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio5, rmt_buffer, &clocks);

    let mut led_red = Output::new(io.pins.gpio8, Level::High);
    let mut led_blue = Output::new(io.pins.gpio9, Level::Low);

    let delay = Delay::new(&clocks);

    loop {
        led_red.toggle();
        led_blue.toggle();
        delay.delay_millis(500);
        led_red.toggle();
        led_blue.toggle();

        // or using `fugit` duration
        delay.delay(2.secs());
    }
}
