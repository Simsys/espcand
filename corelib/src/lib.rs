#![no_std]

use embassy_sync::{pipe::Pipe, blocking_mutex::raw::NoopRawMutex};

mod utils;
mod socket;

pub use socket::command_parser::*;
pub use socket::command_buffer::*;

pub type WifiPipe = Pipe<NoopRawMutex, 4096>;