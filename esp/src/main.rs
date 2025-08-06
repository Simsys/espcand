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
use embassy_futures::select::{select, Either};
use embassy_time::Instant;
use embedded_can::{Frame, Id};

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
        can_rx_channel,
        can_tx_channel,
        wifi_rx_channel,
        wifi_tx_channel,
        signal_conn_rx,
        signal_conn_tx,
    ) = init();

    spawner.spawn(wifi::connection(controller)).ok();
    spawner.spawn(wifi::net_task(runner)).ok();
    spawner.spawn(wifi::comm(stack, wifi_rx_channel, wifi_tx_channel, signal_conn_tx)).ok();
    spawner.spawn(can::comm(twai, can_rx_channel, can_tx_channel, signal_conn_rx)).ok();

    let mut pfilters: PFilters<10> = PFilters::new();
    let mut nfilters: NFilters<10> = NFilters::new();

    loop {
        let can_receive = async {
            can_rx_channel.receive().await
        };
        let wifi_receive = async {
            wifi_rx_channel.receive().await
        };

        // Wait for both and handle first event 
        match select(can_receive, wifi_receive).await {
            Either::First(com_item) => {
                if let ComItem::ReceivedFrame(frame) = &com_item {
                    let id = match frame.id() {
                        Id::Standard(id) => id.as_raw() as u32,
                        Id::Extended(id) => id.as_raw(),
                    };
                    if !nfilters.check(id) {
                        if pfilters.check(id, Instant::now()) {
                            wifi_tx_channel.send(com_item).await;
                        }
                    }
                }
            }
            Either::Second(com_item) => {
                match com_item {
                    ComItem::ClearFilters => {
                        pfilters.clear();
                        nfilters.clear();
                    }
                    ComItem::Echo | ComItem::Error(_) => wifi_tx_channel.send(com_item).await,
                    ComItem::FrameToSend(_) => can_tx_channel.send(com_item).await,
                    ComItem::NFilter(nfilter) => match nfilters.add(nfilter) {
                        Ok(()) => (),
                        Err(error) => wifi_tx_channel.send(ComItem::Error(error)).await,
                    }
                    ComItem::PFilter(pfilter) => match pfilters.add(pfilter) {
                        Ok(()) => (),
                        Err(error) => wifi_tx_channel.send(ComItem::Error(error)).await,
                    }
                    ComItem::ReceivedFrame(_) => (), // wifi does not receive frames
                }
            }
        };
    }
}

