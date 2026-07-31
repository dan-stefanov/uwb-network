use crate::mac::format::{
    nested_ie,
    serializer::{
        FixDeserializable, FixMaybeDeserializable, FixSerializable, FixSerializationLength,
        NoSpaceLeft, Placeholder, ReadBuffer, WriteBuffer,
    },
};

pub const MAX_LENGTH: u16 = 0x7ff;
pub const MAX_GROUP_ID: u8 = 0xf;
pub const MLME_GROUP_ID: u8 = 0x1;

const TYPE_FLAG: u16 = 1u16 << 15;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Debug, Default)]
pub struct Header(u16);

impl Header {
    pub fn length(&self) -> u16 {
        self.0 & MAX_LENGTH
    }

    pub fn set_length(&mut self, value: u16) {
        assert!(value <= MAX_LENGTH);
        self.0 &= !MAX_LENGTH;
        self.0 |= value;
    }

    pub fn group_id(&self) -> u8 {
        (self.0 >> 11) as u8 & MAX_GROUP_ID
    }

    pub fn set_group_id(&mut self, value: u8) {
        assert!(value <= MAX_GROUP_ID);
        self.0 &= !(u16::from(MAX_GROUP_ID) << 11);
        self.0 |= u16::from(value) << 11;
    }
}

impl FixSerializationLength for Header {
    const SER_LEN: usize = u16::SER_LEN;
}

impl FixSerializable for Header {
    fn serialize(&self, buf: &mut [u8]) {
        let val = self.0 | TYPE_FLAG;
        val.serialize(buf);
    }
}

impl FixMaybeDeserializable for Header {
    fn try_deserialize(buf: &[u8]) -> Option<Self> {
        let val = u16::deserialize(buf);
        if val & TYPE_FLAG != 0 {
            Some(Self(val & !TYPE_FLAG))
        } else {
            None
        }
    }
}

pub struct MlmeIeBuilder<'a> {
    buffer: WriteBuffer<'a>,
    header_field: Placeholder<'a, Header>,
    payload_offset: usize,
}

impl<'a> MlmeIeBuilder<'a> {
    pub fn new(mut buffer: WriteBuffer<'a>) -> Result<Self, NoSpaceLeft> {
        let header_field = buffer.append_placeholder::<Header>()?;
        let payload_offset = buffer.len();
        Ok(Self {
            buffer,
            header_field,
            payload_offset,
        })
    }

    pub fn add_nested_ie(&mut self) -> Result<WriteBuffer<'_>, NoSpaceLeft> {
        let payload_len = self.buffer.len() - self.payload_offset;
        Ok(self
            .buffer
            .nested_buffer(usize::from(MAX_LENGTH) - payload_len))
    }
}

impl<'a> Drop for MlmeIeBuilder<'a> {
    fn drop(&mut self) {
        let payload_len = self.buffer.len() - self.payload_offset;
        // Do not commit empty container
        if payload_len > 0 {
            let mut header: Header = Default::default();
            header.set_group_id(MLME_GROUP_ID);
            header.set_length(unwrap!(payload_len.try_into()));
            self.buffer
                .write_placeholder(&mut self.header_field, header);

            let buffer = core::mem::replace(&mut self.buffer, WriteBuffer::new_empty());
            buffer.commit();
        }
    }
}

pub enum NestedContent<'a> {
    Short { sub_id: u8, payload: ReadBuffer<'a> },
    Long { sub_id: u8, payload: ReadBuffer<'a> },
}

pub struct MlmeIeIter<'a> {
    buffer: ReadBuffer<'a>,
}

impl<'a> MlmeIeIter<'a> {
    pub fn new(buffer: ReadBuffer<'a>) -> Self {
        Self { buffer }
    }

    fn raw_next(&mut self) -> Result<Option<NestedContent<'a>>, NoSpaceLeft> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        if let Some(header) = self.buffer.try_pop_field::<nested_ie::ShortIeHeader>()? {
            let payload = self.buffer.pop_buffer(header.length().into())?;
            Ok(Some(NestedContent::Short {
                sub_id: header.sub_id(),
                payload,
            }))
        } else {
            let header = unwrap!(unwrap!(
                self.buffer.try_pop_field::<nested_ie::LongIeHeader>()
            ));
            let payload = self.buffer.pop_buffer(header.length().into())?;
            Ok(Some(NestedContent::Long {
                sub_id: header.sub_id(),
                payload,
            }))
        }
    }
}

impl<'a> Iterator for MlmeIeIter<'a> {
    type Item = Result<NestedContent<'a>, NoSpaceLeft>;

    fn next(&mut self) -> Option<Self::Item> {
        let val = self.raw_next();
        if val.is_err() {
            self.buffer.clear();
        }
        val.transpose()
    }
}
