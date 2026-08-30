use core::num::NonZeroU16;
use defmt::{info, unwrap, warn};
use uwb_network_phy as phy;
use uwb_network_phy::Phy;
use uwb_network_phy::time::Duration;
use uwb_network_phy_dw3000 as dw3000_phy;
use uwb_network_phy_dw3000::Dw3000Phy;

use crate::stat::LinearRegression;

const MAX_PSDU_SIZE: usize = phy::MAX_PSDU_LENGTH as usize;
const MAGIC_PSDU: [u8; 10] = [0x81, 0x70, 0x6f, 0x5e, 0x4d, 0x3c, 0x2b, 0x1a, 0, 0];

const TURN_AROUND_DELAY: Duration = Duration::RSTU.mul_u32(100);
const SCAN_TIMEOUT: Duration = Duration::RSTU.mul_u32(1_000_000);
const SLOT_DURATION: Duration = Duration::RSTU.mul_u32(10_000);
const RESPONSE_COUNT: i32 = 10;

const BASE_RUN_CONFIG: phy::RunConfig = {
    let mut config = phy::RunConfig::new();
    config.psr = phy::Psr::Symbols1024;
    config.correct_tx_fcs = true;
    config.ranging = true;
    config
};
const SHR_DURATION: Duration = phy::shr_duration(
    BASE_RUN_CONFIG.prf,
    BASE_RUN_CONFIG.sfd_type,
    BASE_RUN_CONFIG.psr,
);

fn make_run_config(channel: phy::Channel) -> phy::RunConfig {
    let mut config = BASE_RUN_CONFIG;
    config.channel = channel;
    config
}

fn is_magic_frame(psdu: &[u8]) -> bool {
    psdu.get(..size_of::<u64>()) == Some(&MAGIC_PSDU[..size_of::<u64>()])
}

pub async fn initiator<IF>(phy: &mut Dw3000Phy<IF>, channel: phy::Channel)
where
    IF: dw3000_phy::interface::Interface,
    IF::Error: defmt::Format,
{
    let run_config = make_run_config(channel);
    unwrap!(phy.start(run_config).await);

    unwrap!(phy.write_tx_buffer(&MAGIC_PSDU).await);

    let now = unwrap!(phy.get_timestamp().await);
    let request_timestamp = (now + TURN_AROUND_DELAY).schedule_align_up();

    info!("Send request");
    unwrap!(
        phy.transmit(request_timestamp, MAGIC_PSDU.len() as u16)
            .await
    );

    let mut last_message_timestamp = request_timestamp;
    let mut cumulative_duration = Duration::ZERO;
    let mut timing_error_regression = LinearRegression::new();
    let mut rx_psdu = [0; MAX_PSDU_SIZE];
    for slot_index in 1..=RESPONSE_COUNT {
        const PREAMBLE_TIMEOUT: NonZeroU16 = NonZeroU16::new(64).unwrap();
        let status = unwrap!(
            phy.receive(
                (last_message_timestamp + SLOT_DURATION).schedule_align_up(),
                Some(PREAMBLE_TIMEOUT),
                SLOT_DURATION - TURN_AROUND_DELAY
            )
            .await
        );
        match status {
            Ok(report) => {
                unwrap!(phy.read_rx_buffer(&mut rx_psdu).await);
                if let Some(cia) = report.cia
                    && is_magic_frame(&rx_psdu[..report.length as usize])
                {
                    let message_timestamp = cia.timestamp - SHR_DURATION;
                    cumulative_duration += message_timestamp - last_message_timestamp;
                    last_message_timestamp = message_timestamp;
                    let ideal_duration = SLOT_DURATION * slot_index as u32;
                    let timing_error_ticks =
                        cumulative_duration.as_ticks() as i64 - ideal_duration.as_ticks() as i64;
                    timing_error_regression
                        .add(slot_index, timing_error_ticks)
                        .expect("response count exceeds regression capacity");
                    info!(
                        "Slot {} timestamp: {}, cumulative duration: {}, timing error: {} ticks",
                        slot_index, message_timestamp, cumulative_duration, timing_error_ticks
                    );
                } else {
                    cumulative_duration += SLOT_DURATION;
                    last_message_timestamp += SLOT_DURATION;
                    warn!("Slot {} had an unexpected magic", slot_index);
                }
            }
            Err(error) => {
                cumulative_duration += SLOT_DURATION;
                last_message_timestamp += SLOT_DURATION;
                warn!("Slot {} RX error: {}", slot_index, error);
            }
        }
    }

    if let Ok(fit) = timing_error_regression.fit()
        && fit.sample_count > 2
    {
        let sample_std =
            libm::sqrtf(fit.mse * fit.sample_count as f32 / (fit.sample_count - 2) as f32);
        let slope = fit.slope / SLOT_DURATION.as_ticks() as f32;
        info!(
            "Timing error estimate: offset={} ticks, slope={} ppm, sample std={} ticks",
            fit.intercept,
            slope * 1.0e6,
            sample_std
        );
    }

    info!("Stop PHY");
    unwrap!(phy.stop().await);
}

pub async fn responder<IF>(phy: &mut Dw3000Phy<IF>, channel: phy::Channel)
where
    IF: dw3000_phy::interface::Interface,
    IF::Error: defmt::Format,
{
    let run_config = make_run_config(channel);
    unwrap!(phy.start(run_config).await);

    let mut rx_psdu = [0; MAX_PSDU_SIZE];
    for _ in 0..1000 {
        let now = unwrap!(phy.get_timestamp().await);
        let scan_start = (now + TURN_AROUND_DELAY).schedule_align_up();

        match unwrap!(phy.receive(scan_start, None, SCAN_TIMEOUT).await) {
            Ok(report) => {
                unwrap!(phy.read_rx_buffer(&mut rx_psdu).await);
                if !is_magic_frame(&rx_psdu[..report.length as usize]) {
                    warn!("Request had an unexpected magic");
                    continue;
                }

                let Some(cia) = report.cia else {
                    warn!("Request CIA failed");
                    continue;
                };

                unwrap!(phy.write_tx_buffer(&MAGIC_PSDU).await);
                let mut slot_start =
                    (cia.timestamp - SHR_DURATION + SLOT_DURATION).schedule_align_up();
                for slot_index in 1..=RESPONSE_COUNT {
                    info!("Send slot {}", slot_index);
                    unwrap!(phy.transmit(slot_start, MAGIC_PSDU.len() as u16).await);
                    slot_start += SLOT_DURATION;
                }
            }
            Err(error) => warn!("Request RX error: {}", error),
        }
    }

    info!("Stop PHY");
    unwrap!(phy.stop().await);
}
