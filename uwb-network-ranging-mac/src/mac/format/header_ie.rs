use crate::mac::format::serializer::{
    FixDeserializable, FixMaybeDeserializable, FixSerializable, FixSerializationLength,
};

pub const MAX_LENGTH: u8 = 0x7f;
pub const MAX_ELEMENT_ID: u8 = 0xff;

const TYPE_FLAG: u16 = 1u16 << 15;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Debug, Default)]
pub struct Header(u16);

impl Header {
    pub fn length(&self) -> u8 {
        self.0 as u8 & MAX_LENGTH
    }

    pub fn set_length(&mut self, value: u8) {
        assert!(value <= MAX_LENGTH);
        self.0 &= !u16::from(MAX_LENGTH);
        self.0 |= u16::from(value);
    }

    pub fn element_id(&self) -> u8 {
        (self.0 >> 7) as u8 & MAX_ELEMENT_ID
    }

    #[allow(clippy::absurd_extreme_comparisons)]
    pub fn set_element_id(&mut self, value: u8) {
        assert!(value <= MAX_ELEMENT_ID);
        self.0 &= !(u16::from(MAX_ELEMENT_ID) << 7);
        self.0 |= u16::from(value) << 7;
    }
}

impl FixSerializationLength for Header {
    const SER_LEN: usize = u16::SER_LEN;
}

impl FixSerializable for Header {
    fn serialize(&self, buf: &mut [u8]) {
        let val = self.0;
        val.serialize(buf);
    }
}

impl FixMaybeDeserializable for Header {
    fn try_deserialize(buf: &[u8]) -> Option<Self> {
        let val = u16::deserialize(buf);
        if val & TYPE_FLAG == 0 {
            Some(Self(val))
        } else {
            None
        }
    }
}
