//! Blinks an LED
//!
//! This assumes that a LED is connected to the pin assigned to `led`. (GPIO0)

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3

#![no_std]
#![no_main]

use core::ops::DerefMut;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Gpio8, Gpio9, Io, Level, Output},
    peripherals::Peripherals,
    prelude::*,
    rmt::Rmt,
    system::SystemControl,
    timer::timg::TimerGroup,
};
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use esp_println::logger::init_logger_from_env;
use front_leds::{blue::BlueLed, red::RedLed};
use log::info;
use smart_leds::{
    brightness,
    colors::WHITE,
    gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB,
};

mod front_leds;
// mod pixel_click; // A sort of framework for the board, not yet ready

#[embassy_executor::task]
async fn run(mut red: RedLed, mut blue: BlueLed) {
    loop {
        info!("Playing with the leds concurrently!");
        red.toggle();
        blue.toggle();
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[main]
async fn main(spawner: Spawner) -> ! {
    init_logger_from_env();
    info!("Init!");

    // Initialize peripherals, clocks, GPIO, etc
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    esp_hal_embassy::init(&clocks, timg0);

    // Init the front board LEDs
    let front_red = RedLed::new(io.pins.gpio8, Level::High);
    let front_blue = BlueLed::new(io.pins.gpio9, Level::Low);
    spawner.spawn(run(front_red, front_blue)).ok();

    // For the LED panel, initialize the RMT (Remote Control Transceiver)
    let rmt = Rmt::new(peripherals.RMT, 80.MHz(), &clocks, None).unwrap();
    // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can
    // be used directly with all `smart_led` implementations
    // Our PixelClick has 36 LEDs
    let rmt_buffer = smartLedBuffer!(36);
    let mut a_led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio5, rmt_buffer, &clocks);

    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data: [RGB<u8>; 36];

    // Global delay
    let delay = Delay::new(&clocks);

    loop {
        // led_red.toggle();
        // led_blue.toggle();

        // delay.delay_millis(1000);

        // led_red.toggle();
        // led_blue.toggle();

        // // or using `fugit` duration
        // delay.delay(1.secs());

        // Iterate over the rainbow!
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
                hsv2rgb(color),
            ];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            a_led
                .write(brightness(gamma(data.iter().cloned()), 15))
                .unwrap();

            // front_red.toggle();
            // front_blue.toggle();
            Timer::after(Duration::from_millis(15)).await;
            // delay.delay_millis(15);
            // or with fugit
            // delay.delay(20.millis());
        }
        // a_led
        //     .write(brightness(data.iter().map(|_| WHITE), 10))
        //     .unwrap();
    }
}
