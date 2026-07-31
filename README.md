# UWB-Network

An HRP UWB radio network stack with many-to-many ranging.
The stack is intended for short-range (< 10 m) networks with a dedicated
coordinator.

The stack is self-sufficient and does not require an auxiliary radio for
management.

When possible, the implementation tries to follow IEEE 802.15.4z beacon-enabled
ranging,
though no compatibility is guaranteed.

## Hardware

The repository targets STM32 controllers and the DWM3000 UWB radio module,
though the uwb-network-ranging-mac crate is intended to be hardware-agnostic.

Functional tests are run on a Nucleo-G431RB board with a DWM3000EVB shield.
Consult your local authorities for the channel and power limits allowed in
your region.

## UWB vs other localization technologies

UWB radios enable two-way ranging between two or more stations using
time-of-flight measurements.

Unlike GNSS or Bluetooth ranging, the UWB PHY uses very high bandwidth
(500 MHz or higher)
and theoretically can resolve arrival paths that are 2 ns (0.6 m) apart.
This allows it to achieve sub-centimeter ranging accuracy under heavy
multipath conditions
such as indoor and urban environments.

However, most regional authorities limit transmission power near thermal
noise floor
(-41.3 dBm/MHz), so reliable operation beyond approximately 10 m is unlikely.

## License

uwb-network is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
