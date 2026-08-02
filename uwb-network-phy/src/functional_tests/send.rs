use crate as phy;
use crate::time::Duration;
use embassy_time::Timer;
use heapless::Vec;

const TX_DELAY: Duration = Duration::RSTU.mul_u32(1000);
const RX_DELAY: Duration = Duration::RSTU.mul_u32(1000);
const RX_TIMEOUT: Duration = Duration::RSTU.mul_u32(1_000_000);

const MAX_PSDU_SIZE: usize = phy::PhrFormat::Standard.max_psdu_length() as usize;
const PHY_CONFIG: phy::Config = {
    let mut config = phy::Config::new();
    config.correct_tx_fcs = true;
    config
};

pub async fn initiator<PHY>(phy: &mut PHY)
where
    PHY: phy::Phy,
    PHY::Instant: defmt::Format,
    PHY::IoError: defmt::Format,
    PHY::DevError: defmt::Format,
{
    info!("Run as initiator");

    info!("Configure");
    phy.start(PHY_CONFIG).await.unwrap();

    let psdu = {
        let mut psdu = Vec::<u8, { MAX_PSDU_SIZE }>::new();
        psdu.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x00, 0x00])
            .unwrap();
        psdu
    };

    phy.write_tx_buffer(psdu.as_slice()).await.unwrap();

    for _ in 0..100 {
        info!("Sent a packet");

        let timestamp = phy.get_timestamp().await.unwrap();
        phy.transmit(
            phy::TxConfig::default(),
            unwrap!(u16::try_from(psdu.len())),
            timestamp + TX_DELAY,
        )
        .await
        .unwrap();
        Timer::after_millis(500).await;
    }
}

pub async fn responder<PHY>(phy: &mut PHY)
where
    PHY: phy::Phy,
    PHY::Instant: defmt::Format,
    PHY::IoError: defmt::Format,
    PHY::DevError: defmt::Format,
{
    info!("Run as responder");

    info!("Configure");
    phy.start(PHY_CONFIG).await.unwrap();

    loop {
        let mut psdu = Vec::<u8, { MAX_PSDU_SIZE }>::new();

        info!("Wait for a packet");

        let timestamp = phy.get_timestamp().await.unwrap();

        let status = phy.receive(timestamp + RX_DELAY, RX_TIMEOUT).await;

        match status {
            Ok(Some(report)) => {
                psdu.resize(report.length.into(), 0).unwrap();
                phy.read_rx_buffer(psdu.as_mut_slice()).await.unwrap();

                info!(
                    "RX frame FCS: {}, data: {=[u8]:02x}",
                    report.fcs_good,
                    psdu.as_slice()
                );
            }
            Ok(None) => {
                info!("RX frame timeout, try again");
            }
            Err(err) => panic!("RX error: {:?}", err),
        };
    }
}
