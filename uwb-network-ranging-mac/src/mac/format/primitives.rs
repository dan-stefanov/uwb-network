use crate::mac::format::serializer::{FixDeserializable, FixSerializable, FixSerializationLength};
impl FixSerializationLength for u8 {
    const SER_LEN: usize = 1;
}

impl FixSerializable for u8 {
    fn serialize(&self, buf: &mut [u8]) {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        buf.copy_from_slice(&self.to_le_bytes());
    }
}

impl FixDeserializable for u8 {
    fn deserialize(buf: &[u8]) -> Self {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        u8::from_le_bytes(unwrap!(buf.try_into()))
    }
}

impl FixSerializationLength for u16 {
    const SER_LEN: usize = 2;
}

impl FixSerializable for u16 {
    fn serialize(&self, buf: &mut [u8]) {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        buf.copy_from_slice(&self.to_le_bytes());
    }
}

impl FixDeserializable for u16 {
    fn deserialize(buf: &[u8]) -> Self {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        u16::from_le_bytes(unwrap!(buf.try_into()))
    }
}

impl FixSerializationLength for u32 {
    const SER_LEN: usize = 4;
}

impl FixSerializable for u32 {
    fn serialize(&self, buf: &mut [u8]) {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        buf.copy_from_slice(&self.to_le_bytes());
    }
}

impl FixDeserializable for u32 {
    fn deserialize(buf: &[u8]) -> Self {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        u32::from_le_bytes(unwrap!(buf.try_into()))
    }
}

impl FixSerializationLength for u64 {
    const SER_LEN: usize = 8;
}

impl FixSerializable for u64 {
    fn serialize(&self, buf: &mut [u8]) {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        buf.copy_from_slice(&self.to_le_bytes());
    }
}

impl FixDeserializable for u64 {
    fn deserialize(buf: &[u8]) -> Self {
        // ieee 802.15.4-2020, ch. 4.3, p. 41
        u64::from_le_bytes(unwrap!(buf.try_into()))
    }
}
