#![no_std]

use embassy_sync::{
    channel::Channel,
    pipe::Pipe, 
    blocking_mutex::raw::NoopRawMutex
};

mod utils;

pub use utils::*;

pub type WifiPipe = Pipe<NoopRawMutex, 4096>;
pub type ComChannel = Channel<NoopRawMutex, ComItem, 128>;