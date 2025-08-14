//! Writes and reads flash memory.
//!
//! See https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-guides/partition-tables.html#built-in-partition-tables

#![no_std]
#![no_main]

use embedded_storage::{ReadStorage, Storage};
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::{
    PARTITION_TABLE_MAX_LEN, read_partition_table, PartitionType, 
    AppPartitionSubType, DataPartitionSubType
};
use esp_hal::main;
use esp_println::println;
use esp_storage::FlashStorage;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let _ = esp_hal::init(esp_hal::Config::default());

    let mut flash = FlashStorage::new();
    println!("Flash size = {}", flash.capacity());

    let mut pt_mem = [0u8; PARTITION_TABLE_MAX_LEN];
    let pt = read_partition_table(&mut flash, &mut pt_mem).unwrap();

    let mut app_desc = [0u8; 256];
    pt
        .find_partition(PartitionType::App(AppPartitionSubType::Factory))
        .unwrap()
        .unwrap()
        .as_embedded_storage(&mut flash)
        .read(32, &mut app_desc)
        .unwrap();

    let nvs = pt
        .find_partition(PartitionType::Data(DataPartitionSubType::Nvs))
        .unwrap()
        .unwrap();
    let mut nvs_partition = nvs.as_embedded_storage(&mut flash);
    println!("NVS partition size = {}", nvs_partition.capacity());
    println!();

    nvs_partition
        .write(0, b"$end\n")
        //.write(0, b"$magic,67a35284e62a4b25\n$end\n")
        .unwrap();

    let mut reread_bytes = [0u8; 29];
    nvs_partition.read(0, &mut reread_bytes).unwrap();
    println!("Read:\n{}", str::from_utf8(&reread_bytes).unwrap());

    println!();
    println!("Reset (CTRL-R in espflash) to re-read the persisted data.");

    loop {}
}