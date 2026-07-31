#![cfg_attr(not(test), no_std)]

use uwb_network_phy as phy;

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod mac;
pub mod psdu;

#[cfg(feature = "functional_tests")]
pub mod functional_tests;

pub use phy::time::{Duration, Instant};
