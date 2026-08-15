#![no_std]

use uwb_network_phy_dw3000 as dev;

pub mod lse_capture;
pub mod stat;

/// Allowed in Japan, indoor and outdoor
pub const UWB_CHANNEL: uwb_network_phy::Channel = uwb_network_phy::Channel::CH_9;

pub fn dw3000_device_config() -> dev::DeviceConfig {
    let mut config = dev::DeviceConfig::default();
    // About 5db below the default value. Sufficient for local tests.
    config.tx_power = dev::TxPowerConfig::new_uniform(dev::TxPower::new(0, 32));
    config
}
