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

mod can;
mod init;
mod wifi;

use corelib::ComItem;
use embassy_executor::Spawner;

use esp_alloc as _;
use esp_backtrace as _;

use init::*;
use corelib::*;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let (
        runner, 
        stack, 
        controller,
        twai,
        wifi_tx_channel,
        can_tx_channel,
        wifi_rx_channel,
        signal_conn_rx,
        signal_conn_tx,
    ) = init();

    spawner.spawn(wifi::connection(controller)).ok();
    spawner.spawn(wifi::net_task(runner)).ok();
    spawner.spawn(wifi::comm(stack, wifi_rx_channel, wifi_tx_channel, signal_conn_tx)).ok();
    spawner.spawn(can::comm(twai, wifi_tx_channel, can_tx_channel, signal_conn_rx)).ok();

    loop {
        let datagram = wifi_rx_channel.receive().await;
        match datagram {
            ComItem::FrameToSend(_) => can_tx_channel.send(datagram).await,
            ComItem::Echo | ComItem::Error(_) => wifi_tx_channel.send(datagram).await,
            _ => wifi_tx_channel.send(ComItem::Error(Error::UnknownCommand)).await,
        }
        
    }
}

