#![no_std]

use embassy_sync::{
    channel::Channel,
    blocking_mutex::raw::NoopRawMutex
};

mod filter;
mod utils;

pub use filter::{NFilters, PFilters};
pub use utils::*;

pub type ComChannel = Channel<NoopRawMutex, ComItem, 128>;