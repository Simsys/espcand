#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    Async,
    timer::timg::TimerGroup,
    twai::{self, filter::SingleStandardFilter, TwaiMode, Twai},
};
use esp_println::println;
use log::info;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run(mut twai: Twai<'static, Async>) {
    info!("start can receive");
    loop {
        let r_frame = twai.receive_async().await;
        match r_frame {
            Ok(frame) => println!("Received a frame: {frame:?}"),
            Err(e) => println!("Got error {:?}", e),
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_println::logger::init_logger_from_env();    

    info!("Init can async!");
   
    let tx_pin = peripherals.GPIO3;
    let rx_pin = peripherals.GPIO2;

    const TWAI_BAUDRATE: twai::BaudRate = twai::BaudRate::B1000K;

    let mut twai_config = twai::TwaiConfiguration::new(
        peripherals.TWAI0,
        rx_pin,
        tx_pin,
        TWAI_BAUDRATE,
        TwaiMode::Normal,
    ).into_async();
    twai_config.set_filter(
        const { SingleStandardFilter::new(b"01010000000", b"x", [b"xxxxxxxx", b"xxxxxxxx"]) },
    );
    let twai: Twai<'_, Async> = twai_config.start();

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    spawner.spawn(run(twai)).ok();

    loop {
        info!("Main loop");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
