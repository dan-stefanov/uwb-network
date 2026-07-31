use crate::mac::format::payload_ie::NestedContent;
use crate::mac::format::serializer::{
    FixDeserializable, FixSerializable, FixSerializationLength, Placeholder, ReadBuffer,
};
use crate::mac::format::{header_ie, payload_ie};

use super::{Address, NoSpaceLeft, PanId, WriteBuffer};

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Debug)]
pub enum FrameType {
    Beacon = 0b000,
    Data = 0b001,
    Ack = 0b010,
    MacCommand = 0b011,
    _Reserved100 = 0b100,
    Multipurpose = 0b101,
    Fragment = 0b110,
    Extended = 0b111,
}

impl FrameType {
    fn from_u8_truncate(value: u8) -> Self {
        unsafe { core::mem::transmute(value & 0b111) }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Header {
    pub frame_type: FrameType,
    pub frame_pending: bool,
    pub ack_request: bool,
    pub sequence_number: Option<u8>,
    pub destination_pan_id: Option<PanId>,
    pub destination_address: Option<Address>,
    pub source_pan_id: Option<PanId>,
    pub source_address: Option<Address>,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
enum AddressMode {
    None = 0b00,
    _Reserved01 = 0b01,
    Short = 0b10,
    Extended = 0b11,
}

impl AddressMode {
    fn from_u8_truncate(value: u8) -> Self {
        unsafe { core::mem::transmute(value & 0b11) }
    }
}

impl From<Option<Address>> for AddressMode {
    fn from(value: Option<Address>) -> Self {
        match value {
            None => Self::None,
            Some(Address::Short(_)) => Self::Short,
            Some(Address::Extended(_)) => Self::Extended,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum FrameVersion {
    _0 = 0b00,
    _1 = 0b01,
    _2 = 0b10,
    _Reserved3 = 0b11,
}

impl FrameVersion {
    fn from_u8_truncate(value: u8) -> Self {
        unsafe { core::mem::transmute(value & 0b11) }
    }
}

#[derive(Default, Clone, Copy)]
struct FrameControl(u16);

impl FrameControl {
    fn frame_type(&self) -> FrameType {
        FrameType::from_u8_truncate((self.0 & 0b111) as u8)
    }

    fn set_frame_type(&mut self, value: FrameType) {
        self.0 &= !0b111u16;
        self.0 |= value as u16;
    }

    fn security_enable(&self) -> bool {
        self.0 >> 3 & 0b1 != 0
    }

    #[allow(dead_code)]
    fn set_security_enable(&mut self, value: bool) {
        self.0 &= !(1u16 << 3);
        self.0 |= (value as u16) << 3;
    }

    fn frame_pending(&self) -> bool {
        self.0 >> 4 & 0b1 != 0
    }

    fn set_frame_pending(&mut self, value: bool) {
        self.0 &= !(1u16 << 4);
        self.0 |= (value as u16) << 4;
    }

    fn ack_request(&self) -> bool {
        self.0 >> 5 & 0b1 != 0
    }

    fn set_ack_request(&mut self, value: bool) {
        self.0 &= !(1u16 << 5);
        self.0 |= (value as u16) << 5;
    }

    fn pan_id_compression(&self) -> bool {
        self.0 >> 6 & 0b1 != 0
    }

    fn set_pan_id_compression(&mut self, value: bool) {
        self.0 &= !(1u16 << 6);
        self.0 |= (value as u16) << 6;
    }

    fn seq_num_suppression(&self) -> bool {
        self.0 >> 8 & 0b1 != 0
    }

    fn set_seq_num_suppression(&mut self, value: bool) {
        self.0 &= !(1u16 << 8);
        self.0 |= (value as u16) << 8;
    }

    fn ie_present(&self) -> bool {
        self.0 >> 9 & 0b1 != 0
    }

    fn set_ie_present(&mut self, value: bool) {
        self.0 &= !(1u16 << 9);
        self.0 |= (value as u16) << 9;
    }

    fn dst_address_mode(&self) -> AddressMode {
        AddressMode::from_u8_truncate((self.0 >> 10 & 0b11) as u8)
    }

    fn set_dst_address_mode(&mut self, value: AddressMode) {
        self.0 &= !(11u16 << 10);
        self.0 |= (value as u16) << 10;
    }

    fn frame_version(&self) -> FrameVersion {
        FrameVersion::from_u8_truncate((self.0 >> 12 & 0b11) as u8)
    }

    fn set_frame_version(&mut self, value: FrameVersion) {
        self.0 &= !(11u16 << 12);
        self.0 |= (value as u16) << 12;
    }

    fn src_address_mode(&self) -> AddressMode {
        AddressMode::from_u8_truncate((self.0 >> 14 & 0b11) as u8)
    }

    fn set_src_address_mode(&mut self, value: AddressMode) {
        self.0 &= !(11u16 << 14);
        self.0 |= (value as u16) << 14;
    }
}

impl FixSerializationLength for FrameControl {
    const SER_LEN: usize = u16::SER_LEN;
}

impl FixSerializable for FrameControl {
    fn serialize(&self, buf: &mut [u8]) {
        self.0.serialize(buf);
    }
}

impl FixDeserializable for FrameControl {
    fn deserialize(buf: &[u8]) -> Self {
        const RESERVED_MASK: u16 = 1u16 << 7;
        let val = u16::deserialize(buf);
        Self(val & !RESERVED_MASK)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum HeaderField {
    FrameType,
    FrameVersion,
    SecurityEnable,
    Addressing,
    SeqNumSuppression,
    IePreset,
    IeHeader,
    RbsDuration,
    FirstRcmSlot,
    RmTable,
}

const HEADER_TERMINATION_1_ID: u8 = 0x7e;
const HEADER_TERMINATION_2_ID: u8 = 0x7f;
const PAYLOAD_TERMINATION_ID: u8 = 0xf;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum BuilderError {
    NoSpaceLeft,
    IllegalValue(HeaderField),
    IncorrectElementOrder,
}

impl From<NoSpaceLeft> for BuilderError {
    fn from(_value: NoSpaceLeft) -> Self {
        Self::NoSpaceLeft
    }
}

#[derive(Clone, Copy, Debug)]
enum State {
    FrameHeader,
    HeaderIe,
    PayloadIe,
    Payload,
}

pub struct FrameV2Builder<'a> {
    buffer: WriteBuffer<'a>,
    frame_control: FrameControl,
    frame_control_field: Placeholder<'a, FrameControl>,
    len: usize,
    state: State,
    next_state: Option<State>,
}

impl<'a> FrameV2Builder<'a> {
    pub fn new(mut buffer: WriteBuffer<'a>, header: Header) -> Result<Self, BuilderError> {
        // Multipurpose frame has different layout
        if header.frame_type > FrameType::MacCommand {
            return Err(BuilderError::IllegalValue(HeaderField::FrameType));
        }

        let equal_pan_id = header.destination_pan_id.is_some()
            && header.destination_pan_id == header.source_pan_id;

        let pan_id_compression = match (
            header.destination_address,
            header.source_address,
            header.destination_pan_id,
            header.source_pan_id,
        ) {
            (None, None, None, None) => false,
            (None, None, Some(_), None) => true,
            (Some(_), None, Some(_), None) => false,
            (Some(_), None, None, None) => true,
            (None, Some(_), None, Some(_)) => false,
            (None, Some(_), None, None) => true,
            (Some(Address::Extended(_)), Some(Address::Extended(_)), Some(_), None) => false,
            (Some(Address::Extended(_)), Some(Address::Extended(_)), None, None) => true,
            (Some(Address::Short(_)), Some(Address::Short(_)), Some(_), Some(_)) => equal_pan_id,
            (Some(Address::Short(_)), Some(Address::Extended(_)), Some(_), Some(_)) => equal_pan_id,
            (Some(Address::Extended(_)), Some(Address::Short(_)), Some(_), Some(_)) => equal_pan_id,
            _ => return Err(BuilderError::IllegalValue(HeaderField::Addressing)),
        };

        // let header_ie_present = !self.header_ies.is_empty();
        // let payload_ie_present = !self.nested_ies.is_empty();
        // let payload_data_preset = self.payload_data.is_some();

        let mut frame_control = FrameControl::default();
        frame_control.set_frame_type(header.frame_type);
        frame_control.set_frame_pending(header.frame_pending);
        frame_control.set_ack_request(header.ack_request);
        frame_control.set_pan_id_compression(pan_id_compression);
        frame_control.set_seq_num_suppression(header.sequence_number.is_none());
        frame_control.set_dst_address_mode(header.destination_address.into());
        frame_control.set_frame_version(FrameVersion::_2);
        frame_control.set_src_address_mode(header.source_address.into());

        let frame_control_field = buffer.append_placeholder::<FrameControl>()?;

        if let Some(seq) = header.sequence_number {
            buffer.append_field::<u8>(seq)?;
        }

        if let Some(pan_id) = header.destination_pan_id {
            buffer.append_field(pan_id)?;
        }

        if let Some(addr) = header.destination_address {
            append_addr(&mut buffer, addr)?;
        }

        // If destination and source PAN ID equal, the later is suppressed
        // See [1] table 7-2, p. 163
        if let Some(pan_id) = header.source_pan_id
            && !pan_id_compression
        {
            buffer.append_field(pan_id)?;
        }

        if let Some(addr) = header.source_address {
            append_addr(&mut buffer, addr)?;
        }

        let len = buffer.len();

        Ok(Self {
            buffer,
            frame_control,
            frame_control_field,
            len,
            state: State::FrameHeader,
            next_state: None,
        })
    }

    fn update_state(&mut self) {
        if self.len != self.buffer.len() {
            self.state = unwrap!(self.next_state);
            self.len = self.buffer.len();

            match self.state {
                State::HeaderIe | State::PayloadIe => {
                    self.frame_control.set_ie_present(true);
                }
                _ => {}
            }
        }
    }

    pub fn add_header_ie(&mut self) -> Result<WriteBuffer<'_>, BuilderError> {
        self.update_state();
        match self.state {
            State::FrameHeader | State::HeaderIe => {
                self.next_state = Some(State::HeaderIe);
                Ok(self.buffer.nested_buffer(usize::MAX))
            }
            _ => Err(BuilderError::IncorrectElementOrder),
        }
    }

    pub fn add_payload_ie(&mut self) -> Result<WriteBuffer<'_>, BuilderError> {
        self.update_state();

        let mut buffer = self.buffer.nested_buffer(usize::MAX);
        match self.state {
            State::FrameHeader | State::HeaderIe => {
                let mut terminator: header_ie::Header = Default::default();
                terminator.set_element_id(HEADER_TERMINATION_1_ID);
                buffer.append_field(terminator)?;
            }
            State::PayloadIe => {}
            _ => return Err(BuilderError::IncorrectElementOrder),
        }
        self.next_state = Some(State::PayloadIe);
        Ok(buffer)
    }

    pub fn add_payload(&mut self) -> Result<WriteBuffer<'_>, BuilderError> {
        self.update_state();
        let mut buffer = self.buffer.nested_buffer(usize::MAX);
        match self.state {
            State::FrameHeader => {}
            State::HeaderIe => {
                let mut terminator: header_ie::Header = Default::default();
                terminator.set_element_id(HEADER_TERMINATION_2_ID);
                buffer.append_field(terminator)?;
            }
            State::PayloadIe => {
                let mut terminator: payload_ie::Header = Default::default();
                terminator.set_group_id(PAYLOAD_TERMINATION_ID);
                buffer.append_field(terminator)?;
            }
            _ => return Err(BuilderError::IncorrectElementOrder),
        };

        self.next_state = Some(State::Payload);
        Ok(buffer)
    }
}

impl<'a> Drop for FrameV2Builder<'a> {
    fn drop(&mut self) {
        self.update_state();
        self.buffer
            .write_placeholder(&mut self.frame_control_field, self.frame_control);

        let buffer = core::mem::replace(&mut self.buffer, WriteBuffer::new_empty());
        buffer.commit();
    }
}

fn append_addr(buffer: &mut WriteBuffer, addr: Address) -> Result<(), NoSpaceLeft> {
    match addr {
        Address::Short(value) => buffer.append_field(value),
        Address::Extended(value) => buffer.append_field(value),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedTermination,
    UnsupportedValue(HeaderField),
}

impl From<NoSpaceLeft> for ParseError {
    fn from(_value: NoSpaceLeft) -> Self {
        ParseError::UnexpectedTermination
    }
}

pub fn parse_frame(mut buffer: ReadBuffer) -> Result<(Header, ContentIterator), ParseError> {
    // TODO: Some type of legacy frames (ver < 2) may ignore some control fields
    let frame_control = buffer.pop_field::<FrameControl>()?;

    if frame_control.frame_type() > FrameType::MacCommand {
        return Err(ParseError::UnsupportedValue(HeaderField::FrameType));
    }

    if frame_control.frame_version() > FrameVersion::_2 {
        return Err(ParseError::UnsupportedValue(HeaderField::FrameVersion));
    }

    if frame_control.security_enable() {
        return Err(ParseError::UnsupportedValue(HeaderField::SecurityEnable));
    }

    let legacy_ver = frame_control.frame_version() <= FrameVersion::_1;
    let (dst_pan_id_preset, src_pan_id_present, src_pan_id_copy) = {
        use AddressMode::{Extended, None, Short};
        match (
            legacy_ver,
            frame_control.dst_address_mode(),
            frame_control.src_address_mode(),
            frame_control.pan_id_compression(),
        ) {
            (true, Short | Extended, Short | Extended, pic) => (true, !pic, pic),
            (true, Short | Extended, None, false) => (true, false, false),
            (true, None, Short | Extended, false) => (false, true, false),
            (true, None, None, false) => (false, false, false),
            (false, None, None, pic) => (pic, false, false),
            (false, Short | Extended, None, pic) => (!pic, false, false),
            (false, None, Short | Extended, pic) => (false, !pic, false),
            (false, Extended, Extended, pic) => (!pic, false, false),
            (false, Short, Short, pic) => (true, !pic, pic),
            (false, Short, Extended, pic) => (true, !pic, pic),
            (false, Extended, Short, pic) => (true, !pic, pic),
            _ => return Err(ParseError::UnsupportedValue(HeaderField::Addressing)),
        }
    };

    if frame_control.seq_num_suppression() && legacy_ver {
        return Err(ParseError::UnsupportedValue(HeaderField::SeqNumSuppression));
    };

    if frame_control.ie_present() && legacy_ver {
        return Err(ParseError::UnsupportedValue(HeaderField::IePreset));
    };

    let seq_num: Option<u8> = if !frame_control.seq_num_suppression() {
        Some(buffer.pop_field::<u8>()?)
    } else {
        None
    };

    let dst_pan_id = if dst_pan_id_preset {
        Some(buffer.pop_field::<PanId>()?)
    } else {
        None
    };

    let dst_addr = pop_addr(&mut buffer, frame_control.dst_address_mode())?;

    let src_pan_id = if src_pan_id_present {
        let pan_id = buffer.pop_field::<PanId>()?;
        Some(pan_id)
    } else if src_pan_id_copy {
        dst_pan_id
    } else {
        None
    };

    let src_addr = pop_addr(&mut buffer, frame_control.src_address_mode())?;

    let header = Header {
        frame_type: frame_control.frame_type(),
        frame_pending: frame_control.frame_pending(),
        ack_request: frame_control.ack_request(),
        sequence_number: seq_num,
        destination_pan_id: dst_pan_id,
        destination_address: dst_addr,
        source_pan_id: src_pan_id,
        source_address: src_addr,
    };

    let iter = ContentIterator::new(buffer, frame_control.ie_present());

    Ok((header, iter))
}

fn pop_addr(buffer: &mut ReadBuffer, mode: AddressMode) -> Result<Option<Address>, ParseError> {
    let addr = match mode {
        AddressMode::Short => Some(Address::Short(buffer.pop_field()?)),
        AddressMode::Extended => Some(Address::Extended(buffer.pop_field()?)),
        _ => None,
    };
    Ok(addr)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentError {
    UnexpectedTermination,
    InvalidHeaderIe,
    InvalidPayloadIe,
}

impl From<NoSpaceLeft> for ContentError {
    fn from(_value: NoSpaceLeft) -> Self {
        ContentError::UnexpectedTermination
    }
}

enum ParserState {
    HeaderIe,
    PayloadIe,
    Payload,
}

pub struct ContentIterator<'a> {
    buffer: ReadBuffer<'a>,
    state: ParserState,
}

pub enum Content<'a> {
    HeaderIe {
        element_id: u8,
        payload: ReadBuffer<'a>,
    },
    PayloadIe {
        group_id: u8,
        payload: ReadBuffer<'a>,
    },
    Payload(ReadBuffer<'a>),
}

impl<'a> ContentIterator<'a> {
    fn new(buffer: ReadBuffer<'a>, ie_present: bool) -> Self {
        let state = if ie_present {
            ParserState::HeaderIe
        } else {
            ParserState::Payload
        };
        Self { buffer, state }
    }

    fn raw_next(&mut self) -> Result<Option<Content<'a>>, ContentError> {
        loop {
            if self.buffer.is_empty() {
                return Ok(None);
            }

            match self.state {
                ParserState::HeaderIe => {
                    let header = self
                        .buffer
                        .try_pop_field::<header_ie::Header>()?
                        .ok_or(ContentError::InvalidHeaderIe)?;
                    let payload = self.buffer.pop_buffer(header.length().into())?;
                    match header.element_id() {
                        HEADER_TERMINATION_1_ID => {
                            self.state = ParserState::PayloadIe;
                            continue;
                        }
                        HEADER_TERMINATION_2_ID => {
                            self.state = ParserState::Payload;
                            continue;
                        }
                        _ => {
                            return Ok(Some(Content::HeaderIe {
                                element_id: header.element_id(),
                                payload,
                            }));
                        }
                    }
                }
                ParserState::PayloadIe => {
                    let header = self
                        .buffer
                        .try_pop_field::<payload_ie::Header>()?
                        .ok_or(ContentError::InvalidHeaderIe)?;
                    let payload = self.buffer.pop_buffer(header.length().into())?;

                    match header.group_id() {
                        PAYLOAD_TERMINATION_ID => {
                            self.state = ParserState::Payload;
                            continue;
                        }
                        _ => {
                            return Ok(Some(Content::PayloadIe {
                                group_id: header.group_id(),
                                payload,
                            }));
                        }
                    }
                }
                ParserState::Payload => {
                    let payload = unwrap!(self.buffer.pop_buffer(self.buffer.len()));
                    return Ok(Some(Content::Payload(payload)));
                }
            }
        }
    }
}

impl<'a> Iterator for ContentIterator<'a> {
    type Item = Result<Content<'a>, ContentError>;

    fn next(&mut self) -> Option<Self::Item> {
        let val = self.raw_next();
        if val.is_err() {
            self.buffer.clear();
        }
        val.transpose()
    }
}

#[derive(Debug)]
pub enum FlattenContent<'a> {
    HeaderIe {
        element_id: u8,
        payload: ReadBuffer<'a>,
    },
    PayloadIe {
        group_id: u8,
        payload: ReadBuffer<'a>,
    },
    ShortNestedIe {
        sub_id: u8,
        payload: ReadBuffer<'a>,
    },
    LongNestedIe {
        sub_id: u8,
        payload: ReadBuffer<'a>,
    },
    Payload(ReadBuffer<'a>),
}

impl<'a> From<Content<'a>> for FlattenContent<'a> {
    fn from(value: Content<'a>) -> Self {
        match value {
            Content::HeaderIe {
                element_id,
                payload,
            } => FlattenContent::HeaderIe {
                element_id,
                payload,
            },
            Content::PayloadIe { group_id, payload } => {
                FlattenContent::PayloadIe { group_id, payload }
            }
            Content::Payload(payload) => FlattenContent::Payload(payload),
        }
    }
}

impl<'a> From<NestedContent<'a>> for FlattenContent<'a> {
    fn from(value: NestedContent<'a>) -> Self {
        match value {
            NestedContent::Short { sub_id, payload } => {
                FlattenContent::ShortNestedIe { sub_id, payload }
            }
            NestedContent::Long { sub_id, payload } => {
                FlattenContent::LongNestedIe { sub_id, payload }
            }
        }
    }
}

pub fn parse_frame_flatten(
    buffer: ReadBuffer,
) -> Result<(Header, FlattenContentIterator), ParseError> {
    let (header, iter) = parse_frame(buffer)?;
    let nested_iter = FlattenContentIterator::new(iter);
    Ok((header, nested_iter))
}

pub struct FlattenContentIterator<'a> {
    content_iter: ContentIterator<'a>,
    nested_iter: Option<payload_ie::MlmeIeIter<'a>>,
}

impl<'a> FlattenContentIterator<'a> {
    fn new(content_iter: ContentIterator<'a>) -> Self {
        Self {
            content_iter,
            nested_iter: None,
        }
    }
}

impl<'a> Iterator for FlattenContentIterator<'a> {
    type Item = Result<FlattenContent<'a>, ContentError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(iter) = &mut self.nested_iter {
                match iter.next() {
                    Some(Ok(val)) => return Some(Ok(val.into())),
                    Some(Err(err)) => return Some(Err(err.into())),
                    None => {
                        self.nested_iter = None;
                        continue;
                    }
                }
            }
            match self.content_iter.next() {
                Some(Ok(Content::PayloadIe {
                    group_id: payload_ie::MLME_GROUP_ID,
                    payload,
                })) => {
                    self.nested_iter = Some(payload_ie::MlmeIeIter::new(payload));
                    continue;
                }
                Some(Ok(val)) => return Some(Ok(val.into())),
                Some(Err(err)) => return Some(Err(err)),
                None => return None,
            }
        }
    }
}
