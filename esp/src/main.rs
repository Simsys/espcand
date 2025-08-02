//! Embassy DHCP Example
//!
//!
//! Set SSID and PASSWORD env variable before running this example.
//!
//! This gets an ip address via DHCP then performs an HTTP get request to some
//! "random" server
//!
//! Because of the huge task-arena size configured this won't work on ESP32-S2

//% FEATURES: embassy esp-radio esp-radio/wifi esp-hal/unstable
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]
mod macros;

mod init;
mod wifi;

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::{
        blocking_mutex::raw::CriticalSectionRawMutex, 
        watch::{Watch, Sender, Receiver},
};

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal:: {
    Async,
    twai::Twai,
};
use esp_println::println;
use log::info;

use corelib::*;

use init::*;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let (
        runner, 
        stack, 
        controller,
        wifi_rx_data,
        _wifi_tx_data,
        twai,
    ) = init();

    spawner.spawn(wifi::connection(controller)).ok();
    spawner.spawn(wifi::net_task(runner)).ok();

    static CONNECTION: Watch<CriticalSectionRawMutex, bool, 1> = Watch::new();
    let set_connection: Sender<'_, CriticalSectionRawMutex, bool, 1> = CONNECTION.sender();
    let connection: Receiver<'_, CriticalSectionRawMutex, bool, 1> = CONNECTION.receiver().unwrap();

    let comm_channel = &*mk_static!(ComChannel, ComChannel::new());
    spawner.spawn(wifi::communication(stack, wifi_rx_data, comm_channel, set_connection)).ok();
    spawner.spawn(run(twai, comm_channel, connection)).ok();

    let mut buf = [0_u8; 4096];
    loop {
        let _n = wifi_rx_data.read(&mut buf).await;
        //wifi_tx_data.write(&mut buf[..n]).await;

    }
}

#[embassy_executor::task]
async fn run(
    mut twai: Twai<'static, Async>, 
    com_channel: &'static ComChannel,
    mut connection: Receiver<'static, CriticalSectionRawMutex, bool, 1>,
) {
    info!("start can receive");
    let mut is_connected = false;
    loop {
        let conn = async {
            connection.changed().await
        };
        let r_frame = async {
            twai.receive_async().await
        };

        match select(conn, r_frame).await {
            Either::First(connected) => {
                is_connected = connected;
            }
            Either::Second(r_frame) => {
                let frame = match r_frame {
                    Err(_) => {
                        println!("Got can bus error");
                        continue;
                    }
                    Ok(esp_frame) => CanFrame::from_frame(esp_frame),
                };
                if is_connected {
                    match com_channel.try_send(ComItem::ReceivedFrame(frame)) {
                        Ok(()) => (),
                        Err(_) => println!("Can Queue Error"),
                    }
                }
            }
        };
    }
}

