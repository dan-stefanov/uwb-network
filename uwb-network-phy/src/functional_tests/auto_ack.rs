use crate as phy;
use crate::time::Duration;
use crate::{PanId, ShortAddress};
use embassy_time::Timer;
use heapless::Vec;

const PAN_ID: PanId = PanId::new(0xCAFE);
const INITIATOR_ADDR: ShortAddress = ShortAddress::new(0x0001);
const RESPONDER_ADDR: ShortAddress = ShortAddress::new(0x0002);

const TX_DELAY: Duration = Duration::RSTU.mul_u32(1000);
const RX_DELAY: Duration = Duration::RSTU.mul_u32(1000);
const RX_TIMEOUT: Duration = Duration::RSTU.mul_u32(1_000_000);
const ACK_TIMEOUT: Duration = Duration::RSTU.mul_u32(10_000);

const MAX_PSDU_SIZE: usize = phy::PhrFormat::Standard.max_psdu_length() as usize;

/// Build an IEEE 802.15.4 data frame (version 1) with ack request.
///
/// Addressing with PAN ID compression (v1):
///   Dest PAN ID + Dest Short Addr + Src Short Addr (source PAN ID omitted)
fn build_data_frame(psdu: &mut Vec<u8, { MAX_PSDU_SIZE }>, seq_num: u8, payload: &[u8]) {
    let fc: u16 = 0b10_01_10_000_1_1_0_0_001;
    //               |  |  |     | | | |   \__ frame_type = Data (001)
    //               |  |  |     | | | \______ security_enable = 0
    //               |  |  |     | | \________ frame_pending = 0
    //               |  |  |     | \__________ ack_request = 1
    //               |  |  |     \____________ pan_id_compression = 1
    //               |  |  \__________________ dst_addr_mode = Short (10)
    //               |  \_____________________ frame_version = 1 (01)
    //               \________________________ src_addr_mode = Short (10)

    let header_len = 2 + 1 + 2 + 2 + 2; // FC + Seq + DstPAN + DstAddr + SrcAddr
    let total = header_len + payload.len() + usize::from(phy::FCS_LENGTH);

    let mut buf = [0u8; MAX_PSDU_SIZE];
    buf[0] = fc as u8;
    buf[1] = (fc >> 8) as u8;
    buf[2] = seq_num;
    buf[3] = PAN_ID.as_u16() as u8;
    buf[4] = (PAN_ID.as_u16() >> 8) as u8;
    buf[5] = RESPONDER_ADDR.as_u16() as u8;
    buf[6] = (RESPONDER_ADDR.as_u16() >> 8) as u8;
    buf[7] = INITIATOR_ADDR.as_u16() as u8;
    buf[8] = (INITIATOR_ADDR.as_u16() >> 8) as u8;
    buf[9..9 + payload.len()].copy_from_slice(payload);

    // Length written to buffer excludes FCS (hardware auto-appends)
    psdu.clear();
    psdu.extend_from_slice(&buf[..total - usize::from(phy::FCS_LENGTH)])
        .unwrap();
}

pub async fn initiator<PHY>(phy: &mut PHY)
where
    PHY: phy::Phy,
    PHY::IoError: defmt::Format,
    PHY::DevError: defmt::Format,
{
    info!("Run as initiator");

    info!("Configure");
    let config = phy::Config {
        correct_tx_fcs: true,
        ..Default::default()
    };
    phy.start(config).await.unwrap();
    phy.set_pan_address(PAN_ID, INITIATOR_ADDR).await.unwrap();

    let mut psdu = Vec::<u8, { MAX_PSDU_SIZE }>::new();

    for seq in 0u8..100 {
        build_data_frame(&mut psdu, seq, &[0xDE, 0xAD, 0xBE, 0xEF]);

        phy.write_tx_buffer(psdu.as_slice()).await.unwrap();

        let timestamp = phy.get_timestamp().await.unwrap();

        info!("TX data frame seq={}", seq);

        let ack = phy
            .transmit_w4r(
                phy::TxConfig::default(),
                unwrap!(u16::try_from(psdu.len() + usize::from(phy::FCS_LENGTH))),
                timestamp + TX_DELAY,
                ACK_TIMEOUT,
            )
            .await;

        match ack {
            Ok(Some(report)) if report.fcs_good => {
                info!("Got ACK for seq={}", seq);
            }
            Ok(Some(_)) => {
                info!("Got ACK with bad FCS for seq={}", seq);
            }
            Ok(None) => {
                info!("ACK timeout for seq={}", seq);
            }
            Err(err) => panic!("TX/RX error: {:?}", err),
        }

        Timer::after_millis(500).await;
    }
}

pub async fn responder<PHY>(phy: &mut PHY)
where
    PHY: phy::Phy,
    PHY::IoError: defmt::Format,
    PHY::DevError: defmt::Format,
{
    info!("Run as responder");

    info!("Configure");
    let config = phy::Config {
        correct_tx_fcs: true,
        auto_ack: Some(phy::AutoAckConfig::default()),
        ..Default::default()
    };
    phy.start(config).await.unwrap();
    phy.set_pan_address(PAN_ID, RESPONDER_ADDR).await.unwrap();

    let filter = phy::FrameFilter {
        frame_type_filter: phy::FrameTypeFilter::DATA,
        to_pan_coordinator: false,
        implicit_broadcast: false,
    };
    phy.set_frame_filter(Some(filter)).await.unwrap();

    loop {
        let mut psdu = Vec::<u8, { MAX_PSDU_SIZE }>::new();

        info!("Wait for a data frame");

        let timestamp = phy.get_timestamp().await.unwrap();

        let status = phy.receive(timestamp + RX_DELAY, RX_TIMEOUT).await;

        match status {
            Ok(Some(report)) => {
                psdu.resize(report.length.into(), 0).unwrap();
                phy.read_rx_buffer(psdu.as_mut_slice()).await.unwrap();

                info!(
                    "RX data frame: {=[u8]:02x}, fcs_good={}, ack_sent={}",
                    psdu.as_slice(),
                    report.fcs_good,
                    report.imm_ack
                );
            }
            Ok(None) => {
                info!("RX timeout, try again");
            }
            Err(err) => panic!("RX error: {:?}", err),
        };
    }
}
