use embedded_can::Frame;

use embassy_futures::select::{select3, Either3};
use embassy_sync::{
        blocking_mutex::raw::CriticalSectionRawMutex, 
        watch::Receiver,
};

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal:: {
    twai::{EspTwaiFrame, Twai}, Async
};
use esp_println::println;
use log::info;

use corelib::*;

#[embassy_executor::task]
pub async fn comm(
    mut twai: Twai<'static, Async>, 
    wifi_tx_channel: &'static ComChannel,
    can_tx_channel: &'static ComChannel,
    mut connection: Receiver<'static, CriticalSectionRawMutex, bool, 1>,
) {
    info!("start can receive");
    let mut is_connected = false;
    loop {
        let conn = async {
            connection.changed().await
        };
        let rx_frame = async {
            twai.receive_async().await
        };
        let tx_frame = async {
            can_tx_channel.receive().await
        };

        match select3(conn, rx_frame, tx_frame).await {
            Either3::First(connected) => {
                is_connected = connected;
            }
            Either3::Second(rx_frame) => {
                let frame = match rx_frame {
                    Err(_) => {
                        println!("Got can bus error");
                        continue;
                    }
                    Ok(esp_frame) => CanFrame::from_frame(esp_frame),
                };
                if is_connected {
                    match wifi_tx_channel.try_send(ComItem::ReceivedFrame(frame)) {
                        Ok(()) => (),
                        Err(_) => println!("Can Queue Error"),
                    }
                }
            }
            Either3::Third(tx_frame) => {
                if let ComItem::FrameToSend(can_frame) = tx_frame {
                    let frame = if can_frame.is_remote_frame() {
                        EspTwaiFrame::new_remote(can_frame.id(), can_frame.dlc()).unwrap()
                    } else {
                        EspTwaiFrame::new(can_frame.id(), can_frame.data()).unwrap()
                    };
                    println!("{:?}", &frame);
                    match twai.transmit_async(&frame).await {
                        Ok(()) => (),
                        Err(_) => println!("Could not send can frame"),
                    }
                }
            }
        };
    }
}

