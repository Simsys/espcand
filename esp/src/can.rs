use embedded_can::Frame;

use embassy_futures::select::{select4, Either4};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    peripherals::{GPIO2, GPIO3, TWAI0},
    twai::{EspTwaiFrame, TimingConfig, Twai, BaudRate, TwaiConfiguration, TwaiMode},
    Async,
};
use log::{error, info};

use crate::ComChannel;
use corelib::*;

type RxPin = GPIO2<'static>;
type TxPin = GPIO3<'static>;
type Twai0 = TWAI0<'static>;

pub struct Can {
    twai: Twai<'static, Async>,
}

impl Can {
    pub fn new(
        _twai: Twai0, 
        _rx_pin: RxPin, 
        _tx_pin: TxPin,
        bit_rate: CanBitRate,
    ) -> Self {
        Self::from_bit_rate(bit_rate)
    }

    pub fn change_bit_rate(self, new_bit_rate: CanBitRate) -> Self {
        self.twai.stop();
        Self::from_bit_rate(new_bit_rate)
    }

    pub fn twai(&mut self) -> &mut Twai<'static, Async> {
        &mut self.twai
    }

    fn from_bit_rate(bit_rate: CanBitRate) -> Self {
        let timing = Self::timing_config(bit_rate);
        let baud_rate = BaudRate::Custom(timing);
        let (peripheral, rx_pin, tx_pin) = unsafe {(
            Twai0::steal(),
            RxPin::steal(),
            TxPin::steal(),
        )};
        let twai_config = TwaiConfiguration::new(
            peripheral,
            rx_pin,
            tx_pin,
            baud_rate,
            TwaiMode::Normal,
        ).into_async();
        let twai = twai_config.start();

        Self {
            twai,
        }
    }

    fn timing_config(bit_rate: CanBitRate) -> TimingConfig {
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
}

#[embassy_executor::task]
pub async fn comm(
    mut can: Can,
    wifi_tx_channel: &'static ComChannel,
    can_tx_channel: &'static ComChannel,
    wifi_connection: &'static Signal<CriticalSectionRawMutex, bool>,
    new_can_bit_rate: &'static Signal<CriticalSectionRawMutex, CanBitRate>,
) {
    info!("start can receive");
    let mut is_connected = false;
    loop {
        let wifi_conn = async { wifi_connection.wait().await };
        let can_bit_rate = async { new_can_bit_rate.wait().await };
        let rx_frame = async { can.twai().receive_async().await };
        let tx_frame = async { can_tx_channel.receive().await };

        match select4(wifi_conn, can_bit_rate, rx_frame, tx_frame).await {
            Either4::First(connected) => {
                is_connected = connected;
            }
            Either4::Second(bit_rate) => {
                can = can.change_bit_rate(bit_rate);
            }
            Either4::Third(rx_frame) => {
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
            Either4::Fourth(tx_frame) => {
                if let ComItem::FrameToSend(can_frame) = tx_frame {
                    let frame = if can_frame.is_remote_frame() {
                        EspTwaiFrame::new_remote(can_frame.id(), can_frame.dlc()).unwrap()
                    } else {
                        EspTwaiFrame::new(can_frame.id(), can_frame.data()).unwrap()
                    };
                    match can.twai().transmit_async(&frame).await {
                        Ok(()) => (),
                        Err(_) => error!("Could not send can frame"),
                    }
                }
            }
        };
    }
}
