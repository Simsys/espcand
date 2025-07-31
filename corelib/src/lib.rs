#![no_std]

use embassy_sync::{pipe::Pipe, blocking_mutex::raw::NoopRawMutex};

mod utils;

pub use utils::*;

pub type WifiPipe = Pipe<NoopRawMutex, 4096>;
