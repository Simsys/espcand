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
use embedded_can::Frame;

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
                    if !nfilters.check(frame.id()) {
                        if pfilters.check(frame.id(), Instant::now()) {
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
                    ComItem::ShowFilters => {
                        for nfilter in nfilters.get_vec_ref() {
                            wifi_tx_channel.send(ComItem::NFilter(nfilter.clone())).await;
                        }
                        for pfilter in pfilters.get_vec_ref() {
                            wifi_tx_channel.send(ComItem::PFilter(pfilter.as_pre_pfilter())).await;
                        }
                    }
                }
            }
        };
    }
}

