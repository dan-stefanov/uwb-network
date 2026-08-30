use defmt::{info, unwrap};
use embassy_time::Timer;
use heapless::Vec;
use uwb_network_phy as phy;
use uwb_network_phy::Phy;
use uwb_network_phy::time::Duration;
use uwb_network_phy_dw3000 as dw3000_phy;
use uwb_network_phy_dw3000::Dw3000Phy;

const TX_DELAY: Duration = Duration::RSTU.mul_u32(1000);
const RX_DELAY: Duration = Duration::RSTU.mul_u32(1000);
const RX_TIMEOUT: Duration = Duration::RSTU.mul_u32(1_000_000);
const MAX_PSDU_SIZE: usize = phy::MAX_PSDU_LENGTH as usize;

fn watts_to_dbm(power_watts: f32) -> f32 {
    10.0 * libm::log10f(power_watts / 1.0e-3)
}

pub async fn initiator<IF>(phy: &mut Dw3000Phy<IF>, channel: phy::Channel)
where
    IF: dw3000_phy::interface::Interface,
    IF::Error: defmt::Format,
{
    let mut run_config = phy::RunConfig::new();
    run_config.channel = channel;
    run_config.correct_tx_fcs = true;
    unwrap!(phy.start(run_config).await);

    let mut psdu = Vec::<u8, MAX_PSDU_SIZE>::new();
    unwrap!(psdu.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x00, 0x00]));
    unwrap!(phy.write_tx_buffer(psdu.as_slice()).await);

    for _ in 0..100 {
        let timestamp = unwrap!(phy.get_timestamp().await).schedule_align_up();
        unwrap!(
            phy.transmit(timestamp + TX_DELAY, unwrap!(u16::try_from(psdu.len())))
                .await
        );
        info!("Sent a packet");
        Timer::after_millis(500).await;
    }

    info!("Stop PHY");
    unwrap!(phy.stop().await);
}

pub async fn responder<IF>(phy: &mut Dw3000Phy<IF>, channel: phy::Channel)
where
    IF: dw3000_phy::interface::Interface,
    IF::Error: defmt::Format,
{
    let mut run_config = phy::RunConfig::new();
    run_config.channel = channel;
    run_config.ranging = true;
    unwrap!(phy.start(run_config).await);

    loop {
        let timestamp = unwrap!(phy.get_timestamp().await).schedule_align_up();
        match unwrap!(phy.receive(timestamp + RX_DELAY, None, RX_TIMEOUT).await) {
            Ok(report) => {
                info!("RX frame: {}", report);
                if report.cia.is_some() {
                    let full_power = unwrap!(unwrap!(phy.get_full_cia_power()));
                    let first_path_power = unwrap!(unwrap!(phy.get_first_path_cia_power()));
                    let first_path_energy = unwrap!(unwrap!(phy.get_first_path_energy()));
                    info!(
                        "Pull CIR power: {:?} dBm, first-path power: {:?} dBm, acc energy {:?}",
                        watts_to_dbm(full_power),
                        watts_to_dbm(first_path_power),
                        first_path_energy,
                    );
                }
            }
            Err(error) => info!("RX error: {}", error),
        }
    }
}
