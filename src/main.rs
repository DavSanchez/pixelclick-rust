//! Blinks an LED
//!
//! This assumes that a LED is connected to the pin assigned to `led`. (GPIO0)

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::{Gpio8, Gpio9, Io, Level, Output},
    peripherals::Peripherals,
    prelude::*,
    rmt::{Channel, Rmt},
    system::SystemControl,
    timer::timg::TimerGroup,
    Blocking,
};
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use esp_println::logger::init_logger;
use log::info;
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};

mod front_leds;
// mod pixel_click; // A sort of framework for the board, not yet ready

#[embassy_executor::task]
async fn front_leds(mut red: Output<'static, Gpio8>, mut blue: Output<'static, Gpio9>) {
    loop {
        info!("Playing with the leds concurrently!");
        red.toggle();
        blue.toggle();
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[embassy_executor::task]
async fn led_panel(mut a_led: SmartLedsAdapter<Channel<Blocking, 0>, 865>) {
    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;

    loop {
        info!("Iterating over the rainbow!");
        // Iterate over the rainbow!
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [hsv2rgb(color); 36];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            a_led
                .write(brightness(gamma(data.iter().cloned()), 15))
                .unwrap();

            Timer::after(Duration::from_millis(15)).await;
        }
    }
}

#[main]
async fn main(spawner: Spawner) -> ! {
    init_logger(log::LevelFilter::Info);
    info!("Init!");

    // Initialize peripherals, clocks, GPIO, etc
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    esp_hal_embassy::init(&clocks, timg0);

    // Init the front board LEDs
    let front_red = Output::new(io.pins.gpio8, Level::High);
    let front_blue = Output::new(io.pins.gpio9, Level::Low);

    // For the LED panel, initialize the RMT (Remote Control Transceiver)
    let rmt = Rmt::new(peripherals.RMT, 80.MHz(), &clocks, None).unwrap();
    // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can
    // be used directly with all `smart_led` implementations
    // Our PixelClick has 36 LEDs
    let rmt_buffer = smartLedBuffer!(36);
    let a_led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio5, rmt_buffer, &clocks);

    // Spawn the two tasks!
    spawner.spawn(front_leds(front_red, front_blue)).ok();
    spawner.spawn(led_panel(a_led)).ok();

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
