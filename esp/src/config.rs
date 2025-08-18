
use core::num::ParseIntError;

use corelib::RxBuffer;
use embedded_storage::{ReadStorage, Storage};
use esp_println::print;
use esp_storage::FlashStorage;

use corelib::*;
use log::{info, error};
use crate::init::ComChannel;

const CONF_BUFFER_SIZE: usize = 1024;
const NVS_BASE_ADDRESS: Result<u32, ParseIntError> = u32::from_str_radix(env!("NVS_BASE_ADDRESS"), 16);


pub struct Config {
    flash: FlashStorage,
    base_address: u32,
}

impl Config {
    pub fn new(flash: FlashStorage) -> Self {
        let base_address = NVS_BASE_ADDRESS.unwrap_or(0x9000);
        Self { flash, base_address }
    }

    pub async fn load(&mut self, wifi_rx_channel: &'static ComChannel) {
        let mut buf = RxBuffer::<CONF_BUFFER_SIZE>::default();
        self.flash.read(self.base_address, buf.en_mut_block()).unwrap();
        buf.set_head(CONF_BUFFER_SIZE);

        info!("Config read()", );
        let mut go_on = true;
        let mut magic_detected = false;
        while go_on {
            let mut de_ser = DeSer::<50>::default();
            match buf.read(&mut de_ser) {
                Ok(()) => (),
                Err(_) => break,
            }
            match de_ser.as_slice() {
                MAGIC_DATAGRAM => magic_detected = true,
                b"$end\n" => go_on = false,
                _ => (),
            }

            if magic_detected {
                print!("  {}", str::from_utf8(de_ser.as_slice()).unwrap());
                if let Ok(item) = ComItem::deserialize(&mut de_ser) {
                    wifi_rx_channel.send(item).await;
                };
            }
        }
    }

    pub fn write(&mut self, tx_buf: &mut RxBuffer<CONF_BUFFER_SIZE>) {
        info!("Config write");
        print!("{}", str::from_utf8(&tx_buf.en_mut_block()).unwrap());
        match self.flash.write(self.base_address, &tx_buf.en_mut_block()) {
            Ok(()) => (),
            Err(e) => error!("{:?}", e),
        }
    }
}

pub struct ConfigBuffer {
    buf: RxBuffer<CONF_BUFFER_SIZE>,
}

impl Default for ConfigBuffer {
    fn default() -> Self {
        let mut buf = RxBuffer::<CONF_BUFFER_SIZE>::default();
        buf.write(&ComItem::Magic(true).serialize()).unwrap();
        ConfigBuffer { buf }
    }

}

impl ConfigBuffer {
    pub fn add_item(&mut self, item: &ComItem) -> Result<(), Error> {
        self.buf.write(&item.serialize())?;
        Ok(())
    }

    pub fn finish(&mut self, config: &mut Config) -> Result<(), Error> {
        self.buf.write(&ComItem::End.serialize())?;
        config.write(&mut self.buf);
        Ok(())
    }
}