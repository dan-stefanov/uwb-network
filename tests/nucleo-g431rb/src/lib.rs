#![no_std]

pub mod lse_capture;
pub mod stat;

use uwb_network_phy_dw3000::DeviceConfig;

pub fn dw3000_device_config() -> DeviceConfig {
    use uwb_network_phy_dw3000::*;

    let mut config = DeviceConfig::default();
    config.channel = Channel::Ch9; // Allowed in Japan, indoor and outdoor
    config.rx_ops = RxOps::LongPreamble; // Optimized for 256 symbol preamble or longer
    config.pac = PreambleAcquisitionChunk::Symbols8; // For 64 symbol preamble or longer
    // About 5db below the default value. Sufficient for local tests.
    config.tx_power = TxPowerConfig::new_uniform(TxPower::new(0, 32));
    config
}
