pub mod frame;
pub mod header_ie;
pub mod nested_ie;
pub mod payload_ie;
mod primitives;
pub mod serializer;

pub use serializer::{NoSpaceLeft, WriteBuffer};

use crate::mac::format::serializer::{FixDeserializable, FixSerializable, FixSerializationLength};
use crate::phy::{ExtendedAddress, PanId, ShortAddress};

impl FixSerializationLength for PanId {
    const SER_LEN: usize = 2;
}

impl FixSerializable for PanId {
    fn serialize(&self, buf: &mut [u8]) {
        self.as_u16().serialize(buf);
    }
}

impl FixDeserializable for PanId {
    fn deserialize(buf: &[u8]) -> Self {
        Self::new(u16::deserialize(buf))
    }
}

impl FixSerializationLength for ShortAddress {
    const SER_LEN: usize = 2;
}

impl FixSerializable for ShortAddress {
    fn serialize(&self, buf: &mut [u8]) {
        self.as_u16().serialize(buf);
    }
}

impl FixDeserializable for ShortAddress {
    fn deserialize(buf: &[u8]) -> Self {
        Self::new(u16::deserialize(buf))
    }
}

impl FixSerializationLength for ExtendedAddress {
    const SER_LEN: usize = 8;
}

impl FixSerializable for ExtendedAddress {
    fn serialize(&self, buf: &mut [u8]) {
        self.as_u64().serialize(buf);
    }
}

impl FixDeserializable for ExtendedAddress {
    fn deserialize(buf: &[u8]) -> Self {
        Self::new(u64::deserialize(buf))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Address {
    Short(ShortAddress),
    Extended(ExtendedAddress),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::MAX_PSDU_LENGTH;
    use crate::psdu::{PsduContainer, StaticPsdu};
    use core::num::NonZero;
    use nested_ie::rd_ie::RmSubPeriodUsage;

    #[test]
    fn beacon() {
        let header = frame::Header {
            frame_type: frame::FrameType::Beacon,
            frame_pending: false,
            ack_request: false,
            sequence_number: Some(1),
            destination_pan_id: Some(PanId::new(2)),
            destination_address: Some(Address::Short(ShortAddress::new(3))),
            source_pan_id: Some(PanId::new(4)),
            source_address: Some(Address::Short(ShortAddress::new(5))),
        };

        let ranging_descriptor = nested_ie::rd_ie::RangingDescriptor {
            rbs_duration: 500,
            beacon_interval: 50_000,
            first_rcm_slot: None,
        };

        let mut psdu = StaticPsdu::<MAX_PSDU_LENGTH>::new();

        {
            let mut frame_builder =
                frame::FrameV2Builder::new(psdu.write_buffer(), header).unwrap();
            let mut mlme_builder =
                payload_ie::MlmeIeBuilder::new(frame_builder.add_payload_ie().unwrap()).unwrap();

            let mut rd_builder = nested_ie::rd_ie::RangingDescriptorBuilder::new(
                mlme_builder.add_nested_ie().unwrap(),
                ranging_descriptor,
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

        let golden_output = [
            0x00, 0xaa, 0x01, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, // MAC Header
            0x00, 0x3f, // HT1 IE
            0x0f, 0x88, // MLME IE Start
            0x0d, 0x5b, // Ranging Descriptor IE Start
            0xe8, 0x03, 0x50, 0xc3, 0x00, 0x00, 0x02, // Ranging Descriptor
            0x01, 0x80, 0x00, // RM Table entry 1
            0x11, 0xc0, 0x40, // RM Table entry 0
        ];

        assert_eq!(psdu.as_slice(), &golden_output);

        let (test_header, mut contents) = frame::parse_frame_flatten(psdu.read_buffer()).unwrap();
        assert_eq!(test_header, header);

        match contents.next().unwrap().unwrap() {
            frame::FlattenContent::ShortNestedIe {
                sub_id: nested_ie::rd_ie::SUB_ID,
                payload,
            } => {
                let (test_rd, mut elements) = nested_ie::rd_ie::parse(payload).unwrap();
                assert_eq!(test_rd.rbs_duration, ranging_descriptor.rbs_duration);
                assert_eq!(test_rd.beacon_interval, ranging_descriptor.beacon_interval);
                let element = elements.next().unwrap();
                assert_eq!(
                    (element.start(), element.end(), element.usage()),
                    (1, 16, RmSubPeriodUsage::Rcap)
                );
                let element = elements.next().unwrap();
                assert_eq!(
                    (element.start(), element.end(), element.usage()),
                    (17, 24, RmSubPeriodUsage::Rcfp)
                );
                assert!(elements.next().is_none());
            }
            _ => {
                panic!("Unexpected content");
            }
        }

        assert!(contents.next().is_none());
    }
}
