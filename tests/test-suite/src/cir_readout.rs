use defmt::{info, unwrap, warn};
use embassy_time::Timer;
use embedded_io_async::Write;
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

/// Fixed random value at the start of each decoded CIR record.
pub const CIR_MAGIC: u64 = 0xd2d8_49a7_1e10_c9a1;

#[repr(u32)]
pub enum MessageType {
    Cir = 1,
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

pub async fn responder<IF, W>(phy: &mut Dw3000Phy<IF>, channel: phy::Channel, writer: &mut W)
where
    IF: dw3000_phy::interface::Interface,
    IF::Error: defmt::Format,
    W: Write,
    W::Error: defmt::Format,
{
    let mut run_config = phy::RunConfig::new();
    run_config.channel = channel;
    run_config.ranging = true;
    unwrap!(phy.start(run_config).await);

    loop {
        let timestamp = unwrap!(phy.get_timestamp().await).schedule_align_up();
        match unwrap!(phy.receive(timestamp + RX_DELAY, None, RX_TIMEOUT).await) {
            Ok(report) => {
                info!("Received a frame: {}", report);
                if report.cia.is_some() {
                    readout_cir(phy, writer).await;
                } else {
                    warn!("RX frame without CIA result");
                }
            }
            Err(error) => info!("RX error: {}", error),
        }
    }
}

async fn readout_cir<IF, W>(phy: &mut Dw3000Phy<IF>, writer: &mut W)
where
    IF: dw3000_phy::interface::Interface,
    IF::Error: defmt::Format,
    W: Write,
    W::Error: defmt::Format,
{
    const MAX_CIR_LENGTH: u16 = 1016;
    const MAX_COBS_MESSAGE_LENGTH: usize = {
        const CIR_SAMPLE_SIZE: usize = 2 * size_of::<i32>();
        const CIR_RECORD_LENGTH: usize = size_of::<u64>()
            + size_of::<u32>()
            + size_of::<u16>()
            + MAX_CIR_LENGTH as usize * CIR_SAMPLE_SIZE;

        cobs::max_encoding_length(CIR_RECORD_LENGTH) + 1
    };

    let cir_length = unwrap!(unwrap!(phy.get_cir_length()));
    assert!(cir_length <= MAX_CIR_LENGTH);
    let mut encoded = [0; MAX_COBS_MESSAGE_LENGTH];
    let mut encoder = cobs::CobsEncoder::new(&mut encoded);
    unwrap!(encoder.push(&CIR_MAGIC.to_le_bytes()));
    unwrap!(encoder.push(&(MessageType::Cir as u32).to_le_bytes()));
    unwrap!(encoder.push(&cir_length.to_le_bytes()));

    for index in 0..cir_length {
        let sample = unwrap!(unwrap!(phy.get_cir_sample(index)));
        unwrap!(encoder.push(&sample.re.to_le_bytes()));
        unwrap!(encoder.push(&sample.im.to_le_bytes()));
    }

    let encoded_length = encoder.finalize();
    encoded[encoded_length] = 0;
    unwrap!(writer.write_all(&encoded[..encoded_length + 1]).await);
    unwrap!(writer.flush().await);
}
