#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::info;
use panic_rtt_target as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    timer::timg::TimerGroup,
    gpio::{Level, Output, OutputConfig},
};


// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run(mut led: Output<'static>) {
    loop {
        info!("Hello world from embassy using esp-hal-async!");
        led.toggle();
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Init!");

    let led: Output<'_> = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);


    spawner.spawn(run(led)).ok();

    loop {
        info!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
