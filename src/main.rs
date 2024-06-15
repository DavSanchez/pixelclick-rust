#![no_std]
#![no_main]

// #############################################################
// Basic dependencies
// #############################################################
// Backtrace for ESP32, panic handlers, etc.
use esp_backtrace as _;
// ESP32 HAL (Hardware Abstraction Layer). Provides structures mapping to the actual devices (I/O pins, etc)
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
// #############################################################

// #############################################################
// Async programming with `embassy-rs`
// #############################################################
// Async executor
use embassy_executor::Spawner;
// Async time APIs
use embassy_time::{Duration, Timer};
// #############################################################

// #############################################################
// LED panel management
// #############################################################
// Library abstracting over the addressable LED panel for ESP.
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};
// #############################################################

// #############################################################
// Logging
// #############################################################
// Logger for using the logging macros from `log`.
use esp_println::logger::init_logger;
use log::info;
// #############################################################

// TODO: Ignore this for now!
// mod front_leds;
// mod pixel_click; // A sort of framework for the board, not yet ready

// We had `no_main` put above. We mark this function as the entry point for the async
// runtime. In a way similar to #[tokio::main]
// It's typical that the main function never ends in embedded programs, which keep looping. Hence,
// we can use the "never type" (`!`) as the return value for this function!
#[main]
async fn main(spawner: Spawner) -> ! {
    // Initialize the logger with the default level. Now we can use the typical macros from `log`
    // such as `info!`, `debug!` and so on.
    init_logger(log::LevelFilter::Info);
    info!("Init!");

    // Pretty important step!
    // Initialize peripherals, clocks, GPIO, etc
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // The timer group is needed for initializing the embassy runtime
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    // The IO object is important because it allows us to control each pin of the GPIO (General
    // Purpose I/O). See below"
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Initialize embassy
    esp_hal_embassy::init(&clocks, timg0);

    // Init the front board LEDs as Outputs and with their initial level
    // RED led is in GPIO 8 (see the pinout diagram). We instruct it to start active (HIGH).
    let front_red = Output::new(io.pins.gpio8, Level::High);
    // RED led is in GPIO 9 (see the pinout diagram). We instruct it to start inactive (LOW).
    let front_blue = Output::new(io.pins.gpio9, Level::Low);

    // For the LED panel, initialize the RMT (Remote Control Transceiver)
    let rmt = Rmt::new(peripherals.RMT, 80.MHz(), &clocks, None).unwrap();
    // We use one of the RMT channels to instantiate a `SmartLedsAdapter` which can
    // be used directly with all `smart_led` implementations
    // Our PixelClick has 36 LEDs, so we pass 36 to the below macro to initialize them all.
    let rmt_buffer = smartLedBuffer!(36);
    let a_led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio5, rmt_buffer, &clocks);

    // Spawn the two tasks! See below for the implementations.
    spawner.spawn(back_leds(front_red, front_blue)).ok();
    spawner.spawn(led_panel(a_led)).ok();

    // The tasks are async, so now we just loop indefinitely and that's it!
    // If we were not using async, we would do everything inside this loop (toggle the back LEDs,
    // draw a pattern with the LED panel or drive them individually, etc)
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

// #############################################################
// Below are two async tasks we call from main.
// #############################################################

/// Alternate the LEDs between on/off
#[embassy_executor::task]
async fn back_leds(mut red: Output<'static, Gpio8>, mut blue: Output<'static, Gpio9>) {
    // The task runs indefinitely
    loop {
        info!("Playing with the leds concurrently!");
        // `toggle` passes from HIGH -> LOW or LOW -> HIGH. Just like that!
        red.toggle();
        blue.toggle();
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

/// Emit a rainbow pattern on the front LED panel
#[embassy_executor::task]
async fn led_panel(mut a_led: SmartLedsAdapter<Channel<Blocking, 0>, 865>) {
    // We use the Hsv structure instead of an RGB object because we can change the color by
    // manipulating the `hue` field (using RGB we would need to change three values to do the
    // rainbow effect).
    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;

    // The task runs indefinitely
    loop {
        info!("Iterating over the rainbow!");
        // Iterate over the rainbow!
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED.

            // The syntax used below initializes the 36 items of the array to the same value.
            data = [hsv2rgb(color); 36];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 15 out of 255 so
            // that the output it's not too bright.
            a_led
                .write(brightness(gamma(data.iter().cloned()), 15))
                .unwrap();

            Timer::after(Duration::from_millis(15)).await;
        }
    }
}
