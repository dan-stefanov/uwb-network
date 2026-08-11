#![no_std]

use uwb_network_phy as phy;
use uwb_network_phy_dw3000 as dev;

pub mod lse_capture;
pub mod stat;

pub fn dw3000_device_config() -> dev::DeviceConfig {
    let mut config = dev::DeviceConfig::default();
    config.rx_ops = dev::RxOps::LongPreamble; // Optimized for 256 symbol preamble or longer
    config.pac = dev::PreambleAcquisitionChunk::Symbols8; // For 64 symbol preamble or longer
    config.channel = phy::Channel::CH_9; // Allowed in Japan, indoor and outdoor
    // About 5db below the default value. Sufficient for local tests.
    config.tx_power = dev::TxPowerConfig::new_uniform(dev::TxPower::new(0, 32));
    config
}
