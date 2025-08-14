
use corelib::RxBuffer;
use embedded_storage::{ReadStorage, Storage};
use esp_bootloader_esp_idf::partitions::{
    read_partition_table, AppPartitionSubType, DataPartitionSubType, PartitionType, PARTITION_TABLE_MAX_LEN
};
use esp_println::print;
use esp_storage::FlashStorage;

use corelib::*;
use log::{info, error};
use crate::init::ComChannel;

const CONF_BUFFER_SIZE: usize = 128;

pub struct Config {
    flash: FlashStorage,
}

impl Config {
    pub fn new(flash: FlashStorage) -> Self {
        Self { flash }
    }

    pub async fn load(&mut self, wifi_rx_channel: &'static ComChannel) {
        let mut pt_mem = [0u8; PARTITION_TABLE_MAX_LEN];
        let pt = read_partition_table(&mut self.flash, &mut pt_mem).unwrap();

        let mut app_desc = [0u8; 256];
        pt
            .find_partition(PartitionType::App(AppPartitionSubType::Factory))
            .unwrap()
            .unwrap()
            .as_embedded_storage(&mut self.flash)
            .read(32, &mut app_desc)
            .unwrap();

        let mut buf = RxBuffer::<CONF_BUFFER_SIZE>::default();
        let nvs = pt
            .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
            .unwrap()
            .unwrap();
        let mut nvs_partition = nvs.as_embedded_storage(&mut self.flash);
        nvs_partition
            .read(0, &mut buf.en_mut_block())
            .unwrap();

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
        let mut pt_mem = [0u8; PARTITION_TABLE_MAX_LEN];
        let pt = read_partition_table(&mut self.flash, &mut pt_mem).unwrap();

        let mut app_desc = [0u8; 256];
        pt
            .find_partition(PartitionType::App(AppPartitionSubType::Factory))
            .unwrap()
            .unwrap()
            .as_embedded_storage(&mut self.flash)
            .read(32, &mut app_desc)
            .unwrap();

        let nvs = pt
            .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
            .unwrap()
            .unwrap();
        let mut nvs_partition = nvs.as_embedded_storage(&mut self.flash);

        info!("Config write");
        print!("{}", str::from_utf8(&tx_buf.en_mut_block()).unwrap());
        match nvs_partition.write(0, &tx_buf.en_mut_block()) {
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