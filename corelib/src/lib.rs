#![no_std]

use embassy_sync::{
    channel::Channel,
    blocking_mutex::raw::NoopRawMutex
};

mod utils;

pub use utils::*;

pub type ComChannel = Channel<NoopRawMutex, ComItem, 128>;