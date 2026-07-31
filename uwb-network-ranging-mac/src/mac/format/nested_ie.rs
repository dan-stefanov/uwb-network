use crate::mac::format::serializer::{
    FixDeserializable, FixMaybeDeserializable, FixSerializable, FixSerializationLength,
};

pub mod rd_ie;

const MAX_SHORT_LENGTH: u8 = 0xff;
const MAX_SHORT_SUB_ID: u8 = 0x7f;

const MAX_LONG_LENGTH: u16 = 0x7ff;
const MAX_LONG_SUB_ID: u8 = 0xf;

const TYPE_FLAG: u16 = 1u16 << 15;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Debug, Default)]
pub struct ShortIeHeader(u16);

impl ShortIeHeader {
    pub fn length(&self) -> u8 {
        self.0 as u8 & MAX_SHORT_LENGTH
    }

    pub fn set_length(&mut self, value: u8) {
        self.0 &= !u16::from(MAX_SHORT_LENGTH);
        self.0 |= u16::from(value);
    }

    pub fn sub_id(&self) -> u8 {
        (self.0 >> 8) as u8 & MAX_SHORT_SUB_ID
    }

    pub fn set_sub_id(&mut self, value: u8) {
        assert!(value <= MAX_SHORT_SUB_ID);
        self.0 &= !(u16::from(MAX_SHORT_SUB_ID) << 8);
        self.0 |= u16::from(value) << 8;
    }
}

impl FixSerializationLength for ShortIeHeader {
    const SER_LEN: usize = u16::SER_LEN;
}

impl FixSerializable for ShortIeHeader {
    fn serialize(&self, buf: &mut [u8]) {
        self.0.serialize(buf);
    }
}

impl FixMaybeDeserializable for ShortIeHeader {
    fn try_deserialize(buf: &[u8]) -> Option<Self> {
        let val = u16::deserialize(buf);

        if val & TYPE_FLAG == 0 {
            Some(Self(val))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Debug, Default)]
pub struct LongIeHeader(u16);

impl LongIeHeader {
    pub fn length(&self) -> u16 {
        self.0 & MAX_LONG_LENGTH
    }

    pub fn set_length(&mut self, value: u16) {
        assert!(value <= MAX_LONG_LENGTH);
        self.0 &= !MAX_LONG_LENGTH;
        self.0 |= value;
    }

    pub fn sub_id(&self) -> u8 {
        (self.0 >> 8) as u8 & MAX_LONG_SUB_ID
    }

    pub fn set_sub_id(&mut self, value: u8) {
        assert!(value <= MAX_LONG_SUB_ID);
        self.0 &= !(u16::from(MAX_LONG_SUB_ID) << 8);
        self.0 |= u16::from(value) << 8;
    }
}

impl FixSerializationLength for LongIeHeader {
    const SER_LEN: usize = u16::SER_LEN;
}

impl FixSerializable for LongIeHeader {
    fn serialize(&self, buf: &mut [u8]) {
        let val = self.0 | TYPE_FLAG;
        val.serialize(buf);
    }
}

impl FixMaybeDeserializable for LongIeHeader {
    fn try_deserialize(buf: &[u8]) -> Option<Self> {
        let val = u16::deserialize(buf);
        if val & TYPE_FLAG != 0 {
            Some(Self(val & !TYPE_FLAG))
        } else {
            None
        }
    }
}
