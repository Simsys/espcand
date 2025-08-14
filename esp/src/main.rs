#![no_std]
#![no_main]
mod macros;

mod can;
mod config;
mod init;
mod wifi;

use corelib::ComItem;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::Instant;
use embedded_can::Frame;

use esp_alloc as _;
use esp_backtrace as _;

use corelib::*;
use init::*;

use crate::config::ConfigBuffer;

esp_bootloader_esp_idf::esp_app_desc!();
const FILTER_SIZE: usize = 10;

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
        mut config,
    ) = init();

    config.load(wifi_rx_channel).await;

    spawner.spawn(wifi::connection(controller)).ok();
    spawner.spawn(wifi::net_task(runner)).ok();
    spawner
        .spawn(wifi::comm(
            stack,
            wifi_rx_channel,
            wifi_tx_channel,
            signal_conn_tx,
        ))
        .ok();
    spawner
        .spawn(can::comm(
            twai,
            can_rx_channel,
            can_tx_channel,
            signal_conn_rx,
        ))
        .ok();

    let mut pfilters: PFilters<FILTER_SIZE> = PFilters::default();
    let mut nfilters: NFilters<FILTER_SIZE> = NFilters::default();

    loop {
        let can_receive = async { can_rx_channel.receive().await };
        let wifi_receive = async { wifi_rx_channel.receive().await };

        // Wait for both and handle first event
        match select(can_receive, wifi_receive).await {
            Either::First(com_item) => {
                if let ComItem::ReceivedFrame(frame) = &com_item {
                    if !nfilters.check(frame.id()) && pfilters.check(frame.id(), Instant::now()) {
                        wifi_tx_channel.send(com_item).await;
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
                    },
                    ComItem::PFilter(pfilter) => match pfilters.add(pfilter) {
                        Ok(()) => (),
                        Err(error) => wifi_tx_channel.send(ComItem::Error(error)).await,
                    },
                    ComItem::Save => save_config(
                        &pfilters, 
                        &nfilters,
                        &mut config,
                    ).unwrap(),
                    ComItem::ShowFilters => {
                        for nfilter in nfilters.get_vec_ref() {
                            wifi_tx_channel.send(ComItem::NFilter(*nfilter)).await;
                        }
                        for pfilter in pfilters.get_vec_ref() {
                            wifi_tx_channel
                                .send(ComItem::PFilter(pfilter.as_pre_pfilter()))
                                .await;
                        }
                    }
                    // these ComItems are not accepted from wifi
                    ComItem::End | ComItem::Magic(_) | ComItem::ReceivedFrame(_) => (),
                }
            }
        };
    }
}

pub fn save_config(
    pfilters: &PFilters<FILTER_SIZE>, 
    nfilters: &NFilters<FILTER_SIZE>,
    config: &mut config::Config,
) -> Result<(), Error> {
    let mut buf = ConfigBuffer::default();
    for pfilter in pfilters.get_vec_ref() {
        buf.add_item(&ComItem::PFilter(pfilter.as_pre_pfilter()))?;
    }
    for nfilter in nfilters.get_vec_ref() {
        buf.add_item(&ComItem::NFilter(*nfilter))?;
    }
    buf.finish(config)?;
    Ok(())
}