//! This example sends a TWAI message to another ESP and receives it back.
//!
//! `IS_FIRST_SENDER` below must be set to false on one of the ESP's
//!
//! In case you want to use `self-testing`, get rid of everything related to the
//! aforementioned `IS_FIRST_SENDER` and follow the advice in the comments
//! related to this mode.
//!
//! The following wiring is assumed:
//! - TX/RX => GPIO2, connected internally and with internal pull-up resistor.
//!
//! ESP1/GND --- ESP2/GND
//! ESP1/GPIO2 --- ESP2/GPIO2
//!
//! Notes for external transceiver use:
//!
//! The default setup assumes that two microcontrollers are connected directly
//! without an external transceiver. If you want to use an external transceiver,
//! you need to:
//! * uncomment the `rx_pin` line
//! * use `new()` function to create the TWAI configuration.
//! * change the `tx_pin` and `rx_pin` to the appropriate pins for your boards.

//% CHIPS: esp32 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: esp-hal/unstable

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    main,
    twai::{self, filter::SingleStandardFilter, EspTwaiFrame, StandardId, TwaiMode},
};
use esp_println::println;
use nb::block;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let tx_pin = peripherals.GPIO3;
    let rx_pin = peripherals.GPIO2;

    const TWAI_BAUDRATE: twai::BaudRate = twai::BaudRate::B1000K;

    let mut twai_config = twai::TwaiConfiguration::new(
        peripherals.TWAI0,
        rx_pin,
        tx_pin,
        TWAI_BAUDRATE,
        TwaiMode::Normal,
    );

    // Partially filter the incoming messages to reduce overhead of receiving
    // undesired messages. Note that due to how the hardware filters messages,
    // standard ids and extended ids may both match a filter. Frame ids should
    // be explicitly checked in the application instead of fully relying on
    // these partial acceptance filters to exactly match.
    // A filter that matches StandardId::ZERO.
    twai_config.set_filter(
        const { SingleStandardFilter::new(b"01010000000", b"x", [b"xxxxxxxx", b"xxxxxxxx"]) },
    );

    // Start the peripheral. This locks the configuration settings of the peripheral
    // and puts it into operation mode, allowing packets to be sent and
    // received.
    let mut twai = twai_config.start();

    // Set McCready to 0 ( Larus Can Bus Protocol )
    let id = StandardId::new(0x522).unwrap();
    let frame = EspTwaiFrame::new(id, &[1, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    block!(twai.transmit(&frame)).unwrap();

    loop {
        // Wait for a frame to be received.
        let frame = block!(twai.receive()).unwrap();

        println!("Received a frame: {frame:?}");
    }
}
