use crate::mac::format;
use crate::phy;
use crate::phy::time::Duration;
use crate::phy::{BitRate, PreambleLength};
use crate::psdu::{PsduContainer, StaticPsdu};
use core::num::NonZero;
use embassy_time::Timer;

const SLOT_DURATION: Duration = Duration::RSTU.mul_u32(10000);

const TURNAROUND_DURATION: Duration = Duration::RSTU.mul_u32(1000);

const SLOT_COUNT: u8 = 5;

const MAX_PSDU_SIZE: usize = phy::PhrFormat::Standard.max_psdu_length() as usize;

const RX_CONFIG: phy::RxConfig = phy::RxConfig {
    max_preamble_length: PreambleLength::Symbols64,
    max_preamble_hunt: None,
};

const TX_CONFIG: phy::TxConfig = phy::TxConfig {
    preamble_length: PreambleLength::Symbols64,
    bit_rate: BitRate::Kbs850,
    ranging_flag: false,
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

    let preamble_code = phy.preamble_prf().min_code();
    let run_config = phy::RunConfig {
        preamble_code: preamble_code,
        correct_tx_fcs: true,
        ..Default::default()
    };

    phy.start(run_config).await.unwrap();
    assert_eq!(phy.state(), phy::State::Running);

    let shr_duration = phy::shr_duration(
        phy.preamble_prf(),
        phy.sfd_length(),
        TX_CONFIG.preamble_length,
    );

    let mut beacon_psdu = StaticPsdu::<MAX_PSDU_SIZE>::new();
    {
        use format::nested_ie::rd_ie::RmSubPeriodUsage;
        use format::{Address, PanId, ShortAddress};

        let header = format::frame::Header {
            frame_type: format::frame::FrameType::Beacon,
            frame_pending: false,
            ack_request: false,
            sequence_number: Some(1),
            destination_pan_id: Some(PanId::new(2)),
            destination_address: Some(Address::Short(ShortAddress::new(3))),
            source_pan_id: Some(PanId::new(4)),
            source_address: Some(Address::Short(ShortAddress::new(5))),
        };

        let ranging_desc = format::nested_ie::rd_ie::RangingDescriptor {
            rbs_duration: 500,
            beacon_interval: 50_000,
            first_rcm_slot: None,
        };

        let mut frame_builder =
            format::frame::FrameV2Builder::new(beacon_psdu.write_buffer(), header).unwrap();
        let mut mlme_builder =
            format::payload_ie::MlmeIeBuilder::new(frame_builder.add_payload_ie().unwrap())
                .unwrap();

        let mut rd_builder = format::nested_ie::rd_ie::RangingDescriptorBuilder::new(
            mlme_builder.add_nested_ie().unwrap(),
            ranging_desc,
            false,
        )
        .unwrap();

        rd_builder
            .add_management_sub_period(RmSubPeriodUsage::Rcap, NonZero::new(16).unwrap())
            .unwrap();
        rd_builder
            .add_management_sub_period(RmSubPeriodUsage::Rcfp, NonZero::new(8).unwrap())
            .unwrap();
    }

    let mut slot_psdu = StaticPsdu::<MAX_PSDU_SIZE>::new();
    slot_psdu
        .set_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x10, 0x00, 0x00, 0x00, 0x00])
        .unwrap();

    for super_frame_ind in 0..100 {
        info!("Start super frame");

        beacon_psdu.as_mut_slice()[5] = super_frame_ind;
        phy.write_tx_buffer(beacon_psdu.as_slice()).await.unwrap();

        let now = phy.get_timestamp().await.unwrap();
        let super_frame_start = now + TURNAROUND_DURATION;

        phy.transmit(
            TX_CONFIG,
            unwrap!(u16::try_from(beacon_psdu.len())),
            super_frame_start + shr_duration,
        )
        .await
        .unwrap();

        for i in 1..SLOT_COUNT {
            slot_psdu.as_mut_slice()[5] = super_frame_ind;
            slot_psdu.as_mut_slice()[6] = i;
            phy.write_tx_buffer(slot_psdu.as_slice()).await.unwrap();

            phy.transmit(
                TX_CONFIG,
                unwrap!(u16::try_from(slot_psdu.len())),
                super_frame_start + (i as u32) * SLOT_DURATION + shr_duration,
            )
            .await
            .unwrap();
        }
        Timer::after_millis(1500).await;
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
    let preamble_code = phy.preamble_prf().min_code();
    let run_config = phy::RunConfig {
        preamble_code: preamble_code,
        correct_tx_fcs: true,
        ..Default::default()
    };

    phy.start(run_config).await.unwrap();
    assert_eq!(phy.state(), phy::State::Running);

    let shr_duration = phy::shr_duration(
        phy.preamble_prf(),
        phy.sfd_length(),
        TX_CONFIG.preamble_length,
    );

    loop {
        let mut psdu = StaticPsdu::<MAX_PSDU_SIZE>::new();

        info!("Wait for a beacon");
        let now = phy.get_timestamp().await.unwrap();

        let status = phy
            .receive(
                RX_CONFIG,
                now + TURNAROUND_DURATION,
                PHY::MAX_RX_FRAME_TIMEOUT,
            )
            .await
            .unwrap();

        if let Some(report) = status {
            if !report.fcs_good {
                continue;
            }

            psdu.set_length(report.length.into()).unwrap();
            phy.read_rx_buffer(psdu.as_mut_slice()).await.unwrap();
            display_beacon_report(status, psdu.as_slice());

            let super_frame_start = report.timestamp - shr_duration;

            for i in 1..SLOT_COUNT {
                let status = phy
                    .receive(
                        RX_CONFIG,
                        super_frame_start + (i as u32) * SLOT_DURATION,
                        SLOT_DURATION - TURNAROUND_DURATION,
                    )
                    .await
                    .unwrap();

                if let Some(report) = status {
                    psdu.set_length(report.length.into()).unwrap();
                    phy.read_rx_buffer(psdu.as_mut_slice()).await.unwrap();
                    display_slot_report(i, status, psdu.as_slice());
                }
            }
        }

        Timer::after_millis(100).await;
    }
}

fn display_beacon_report<T>(status: Option<phy::RxReport<T>>, psdu: &[u8]) {
    match status {
        Some(report) => {
            if report.fcs_good {
                info!("Beacon frame: {:02x}", psdu);
            } else {
                info!("Beacon frame FCS error");
            }
        }
        None => {
            info!("Beacon frame timeout");
        }
    }
}

fn display_slot_report<T>(slot_idx: u8, status: Option<phy::RxReport<T>>, psdu: &[u8]) {
    match status {
        Some(report) => {
            if report.fcs_good {
                info!("Slot {} frame: {:02x}", slot_idx, psdu);
            } else {
                info!("Slot {} frame FCS error", slot_idx);
            }
        }
        None => {
            info!("Slot {} frame timeout", slot_idx);
        }
    }
}
