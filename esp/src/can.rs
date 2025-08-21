use embedded_can::Frame;

use embassy_futures::select::{select3, Either3};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Receiver};

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    twai::{EspTwaiFrame, TimingConfig, Twai},
    Async,
};
use log::{error, info};

use crate::ComChannel;
use corelib::*;

pub fn timing_config(bit_rate: CanBitRate) -> TimingConfig {
    let baud_rate_prescaler: u16 = match bit_rate {
        CanBitRate::B10k => 400,
        CanBitRate::B20k => 200,
        CanBitRate::B50k => 80,
        CanBitRate::B100k => 40,
        CanBitRate::B125k => 32,
        CanBitRate::B250k => 16,
        CanBitRate::B500k => 8,
        CanBitRate::B1000k => 4,
    };
    TimingConfig {
        baud_rate_prescaler,
        sync_jump_width: 3,
        tseg_1: 15,
        tseg_2: 4,
        triple_sample: false,
    }
}



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
        let conn = async { connection.changed().await };
        let rx_frame = async { twai.receive_async().await };
        let tx_frame = async { can_tx_channel.receive().await };

        match select3(conn, rx_frame, tx_frame).await {
            Either3::First(connected) => {
                is_connected = connected;
            }
            Either3::Second(rx_frame) => {
                let frame = match rx_frame {
                    Ok(esp_frame) => CanFrame::from_frame(esp_frame),
                    Err(_) => {
                        error!("Can Bus Error");
                        continue;
                    }
                };
                if is_connected {
                    match wifi_tx_channel.try_send(ComItem::ReceivedFrame(frame)) {
                        Ok(()) => (),
                        Err(_) => {
                            error!("Can Queue Error");
                            esp_hal::system::software_reset();
                        }
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
                    match twai.transmit_async(&frame).await {
                        Ok(()) => (),
                        Err(_) => error!("Could not send can frame"),
                    }
                }
            }
        };
    }
}
