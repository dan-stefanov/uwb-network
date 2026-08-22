#![no_std]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

mod device;
mod dw3000;
pub mod interface;
mod otp;
#[allow(dead_code)]
mod ral;

pub use device::{TxPower, TxPowerConfig};
pub use dw3000::{DeviceConfig, Dw3000Phy};
use uwb_network_phy as phy;
