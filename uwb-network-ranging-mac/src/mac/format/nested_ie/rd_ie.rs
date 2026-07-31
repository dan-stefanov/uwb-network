use core::num::NonZeroU16;
use core::num::NonZeroU32;

use super::ShortIeHeader;
use crate::mac::format::{
    nested_ie::MAX_SHORT_LENGTH,
    serializer::{
        FixDeserializable, FixSerializable, FixSerializationLength, NoSpaceLeft, Placeholder,
        ReadBuffer, WriteBuffer,
    },
};

pub const SUB_ID: u8 = 0x5b;

pub const MAX_RSB_DURATION: u16 = 0x7fff;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct RangingDescriptor {
    /// ranging beacon slot duration in RSTU, 15 bit max
    pub rbs_duration: u16,
    /// interval between consequent beacon in RSTU
    pub beacon_interval: u32,
    /// start of ranging period if present
    pub first_rcm_slot: Option<NonZeroU32>,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum RmSubPeriodUsage {
    /// Ranging contention access period
    Rcap,
    /// Ranging contention-free period
    Rcfp,
}

enum SlotIndexPlaceholder<'a> {
    Short(Placeholder<'a, u16>),
    Long(Placeholder<'a, u32>),
}

const RBS_INDEX_MAX: u16 = 0x7ff;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct RmTableElement(u32);

impl RmTableElement {
    pub fn start(&self) -> u16 {
        self.0 as u16 & RBS_INDEX_MAX
    }

    pub fn set_start(&mut self, value: u16) {
        assert!(value <= RBS_INDEX_MAX);
        self.0 &= !u32::from(RBS_INDEX_MAX);
        self.0 |= u32::from(value);
    }

    pub fn end(&self) -> u16 {
        (self.0 >> 11) as u16 & RBS_INDEX_MAX
    }

    pub fn set_end(&mut self, value: u16) {
        assert!(value <= RBS_INDEX_MAX);
        self.0 &= !(u32::from(RBS_INDEX_MAX) << 11);
        self.0 |= u32::from(value) << 11;
    }

    pub fn usage(&self) -> RmSubPeriodUsage {
        if self.0 >> 22 & 0x1 == 0 {
            RmSubPeriodUsage::Rcap
        } else {
            RmSubPeriodUsage::Rcfp
        }
    }

    pub fn set_usage(&mut self, value: RmSubPeriodUsage) {
        let val = match value {
            RmSubPeriodUsage::Rcap => 0,
            RmSubPeriodUsage::Rcfp => 1,
        };
        self.0 &= !(1u32 << 22);
        self.0 |= val << 22;
    }
}

impl FixSerializationLength for RmTableElement {
    const SER_LEN: usize = 3;
}

impl FixSerializable for RmTableElement {
    fn serialize(&self, buf: &mut [u8]) {
        buf.copy_from_slice(&self.0.to_le_bytes()[..Self::SER_LEN]);
    }
}

impl FixDeserializable for RmTableElement {
    fn deserialize(buf: &[u8]) -> Self {
        const RESERVED_MASK: u32 = 1u32 << 23;

        let mut bytes = [0u8; 4];
        bytes[..Self::SER_LEN].copy_from_slice(buf);

        // ieee 802.15.4-2020, ch. 4.3, p. 41
        let val = u32::from_le_bytes(bytes);
        Self(val & !RESERVED_MASK)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum BuilderError {
    NoSpaceLeft,
    InvalidRbsDuration,
    BeaconIntervalOverflow,
    RmTableOverflow,
    IncorrectRcmSlotSpec,
}

impl From<NoSpaceLeft> for BuilderError {
    fn from(_value: NoSpaceLeft) -> Self {
        Self::NoSpaceLeft
    }
}

pub struct RangingDescriptorBuilder<'a> {
    buffer: WriteBuffer<'a>,
    header_field: Placeholder<'a, ShortIeHeader>,
    first_rcm_slot_field: SlotIndexPlaceholder<'a>,
    rm_table_length_field: Placeholder<'a, u8>,
    payload_offset: usize,
    ranging_period_present: bool,
    last_rbs_index: u16,
    rbs_index_max: u16,
    rm_table_length: u8,
}

impl<'a> RangingDescriptorBuilder<'a> {
    /// Creates the builder
    ///
    /// desc.first_rcm_slot should be set to None.
    /// If ranging_period_present, the first_rcm_slot field will be calculated accordingly.
    pub fn new(
        mut buffer: WriteBuffer<'a>,
        desc: RangingDescriptor,
        ranging_period_present: bool,
    ) -> Result<Self, BuilderError> {
        let header_field = buffer.append_placeholder::<ShortIeHeader>()?;
        let payload_offset = buffer.len();
        buffer.limit_capacity(payload_offset + usize::from(MAX_SHORT_LENGTH));

        if desc.rbs_duration == 0 || desc.rbs_duration >= MAX_RSB_DURATION {
            return Err(BuilderError::InvalidRbsDuration);
        }

        let beacon_interval_slots = desc.beacon_interval / u32::from(desc.rbs_duration);
        if beacon_interval_slots == 0 {
            return Err(BuilderError::BeaconIntervalOverflow);
        }

        let rbs_index_max =
            core::cmp::min(beacon_interval_slots - 1, u32::from(RBS_INDEX_MAX)) as u16;

        let short_beacon_interval = u16::try_from(desc.beacon_interval).ok();
        buffer.append_field(desc.rbs_duration << 1 | u16::from(short_beacon_interval.is_none()))?;

        if let Some(interval) = short_beacon_interval {
            buffer.append_field::<u16>(interval)?;
        } else {
            buffer.append_field::<u32>(desc.beacon_interval)?;
        }

        if desc.first_rcm_slot.is_some() {
            return Err(BuilderError::IncorrectRcmSlotSpec);
        }

        let first_rcm_slot_field = if short_beacon_interval.is_some() {
            SlotIndexPlaceholder::Short(buffer.append_placeholder()?)
        } else {
            SlotIndexPlaceholder::Long(buffer.append_placeholder()?)
        };

        let rm_table_length_field = buffer.append_placeholder::<u8>()?;

        Ok(Self {
            buffer,
            header_field,
            first_rcm_slot_field,
            rm_table_length_field,
            payload_offset,
            ranging_period_present,
            last_rbs_index: 0, // beacon slot is alway occupied
            rbs_index_max,
            rm_table_length: 0,
        })
    }

    pub fn add_management_sub_period(
        &mut self,
        usage: RmSubPeriodUsage,
        length: NonZeroU16,
    ) -> Result<(), BuilderError> {
        if u16::from(length) > self.rbs_index_max - self.last_rbs_index {
            return Err(BuilderError::BeaconIntervalOverflow);
        }

        let mut elem: RmTableElement = Default::default();
        elem.set_start(self.last_rbs_index + 1);
        elem.set_end(self.last_rbs_index + u16::from(length));
        elem.set_usage(usage);
        self.buffer.append_field(elem)?;

        self.last_rbs_index += u16::from(length);

        // Element count is limited by u8 IE length
        self.rm_table_length += 1;
        Ok(())
    }
}

impl<'a> Drop for RangingDescriptorBuilder<'a> {
    fn drop(&mut self) {
        let payload_len = self.buffer.len() - self.payload_offset;
        let mut header: ShortIeHeader = Default::default();
        header.set_length(unwrap!(u8::try_from(payload_len)));
        header.set_sub_id(SUB_ID);
        self.buffer
            .write_placeholder(&mut self.header_field, header);

        let first_rcm_slot = if self.ranging_period_present {
            self.last_rbs_index + 1
        } else {
            0
        };
        match &mut self.first_rcm_slot_field {
            SlotIndexPlaceholder::Short(field) => {
                self.buffer.write_placeholder(field, first_rcm_slot)
            }
            SlotIndexPlaceholder::Long(field) => {
                self.buffer.write_placeholder(field, first_rcm_slot.into())
            }
        }

        self.buffer
            .write_placeholder(&mut self.rm_table_length_field, self.rm_table_length);

        let buffer = core::mem::replace(&mut self.buffer, WriteBuffer::new_empty());
        buffer.commit();
    }
}

pub fn parse(mut buffer: ReadBuffer) -> Result<(RangingDescriptor, SubPeriodIter), NoSpaceLeft> {
    let word: u16 = buffer.pop_field()?;
    let long_interval = word & 0x1 != 0;
    let rbs_duration = word >> 1;
    let beacon_interval = if long_interval {
        buffer.pop_field::<u32>()?
    } else {
        buffer.pop_field::<u16>()?.into()
    };
    let first_rcm_slot = if long_interval {
        buffer.pop_field::<u32>()?
    } else {
        buffer.pop_field::<u16>()?.into()
    };
    let rm_table_length = buffer.pop_field::<u8>()?;

    let table_len = usize::from(rm_table_length) * RmTableElement::SER_LEN;
    let table_buffer = buffer.pop_buffer(table_len)?;

    let desc = RangingDescriptor {
        rbs_duration,
        beacon_interval,
        first_rcm_slot: NonZeroU32::new(first_rcm_slot),
    };

    let iter = SubPeriodIter {
        buffer: table_buffer,
    };
    Ok((desc, iter))
}

pub struct SubPeriodIter<'a> {
    buffer: ReadBuffer<'a>,
}

impl<'a> Iterator for SubPeriodIter<'a> {
    type Item = RmTableElement;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.pop_field().ok()
    }
}
