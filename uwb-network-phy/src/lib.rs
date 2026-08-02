#![cfg_attr(not(test), no_std)]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod time;

#[cfg(feature = "functional_tests")]
pub mod functional_tests;

use bitflags::bitflags;

pub const FCS_LENGTH: u16 = 2;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Prf {
    Mhz16,
    Mhz64,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PreambleCode {
    /// 16MHz PRF
    Code3 = 3,
    /// 16MHz PRF
    Code4 = 4,
    /// 64MHz PRF
    Code9 = 9,
    /// 64MHz PRF
    Code10 = 10,
    /// 64MHz PRF
    Code11 = 11,
    /// 64MHz PRF
    Code12 = 12,
}

impl PreambleCode {
    pub const fn as_number(self) -> u8 {
        self as u8
    }

    pub const fn prf(self) -> Prf {
        // See tables 15-6 and 15-7 for code length
        // See table 15-4 for allowed PFR for code length
        match self {
            PreambleCode::Code3 => Prf::Mhz16,  // 31 chip code
            PreambleCode::Code4 => Prf::Mhz16,  // 31 chip code
            PreambleCode::Code9 => Prf::Mhz64,  // 123 chip code
            PreambleCode::Code10 => Prf::Mhz64, // 123 chip code
            PreambleCode::Code11 => Prf::Mhz64, // 123 chip code
            PreambleCode::Code12 => Prf::Mhz64, // 123 chip code
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PreambleLength {
    /// IEEE 802.15.4 standard length
    Symbols16,
    /// IEEE 802.15.4 standard length
    Symbols64,
    /// IEEE 802.15.4 standard length
    Symbols1024,
    /// IEEE 802.15.4 standard length
    Symbols4096,
}

impl PreambleLength {
    pub const fn as_symbols(self) -> u16 {
        match self {
            PreambleLength::Symbols16 => 16,
            PreambleLength::Symbols64 => 64,
            PreambleLength::Symbols1024 => 1024,
            PreambleLength::Symbols4096 => 4096,
        }
    }
}

/// Subset of IEEE802.15.4z SFD sequence list, see phyHrpUwbSfdSelector
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SfdType {
    /// 8 symbols, supported by IEEE802.15.4 and IEEE802.15.4z
    Sfd0,
    /// 8 symbols, better SNR, IEEE802.15.4z only
    Sfd2,
}

impl SfdType {
    /// Length of SFD in preamble symbols
    pub const fn symbol_length(self) -> u8 {
        match self {
            SfdType::Sfd0 => 8,
            SfdType::Sfd2 => 8,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BitRate {
    Kbs850,
    Kbs6810,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PhrFormat {
    /// Defined by IEEE 802.15.4, PSDU is 0-127 octets
    Standard,
    /// Defined by IEEE 802.15.8, PSDU is 0-1023 octets
    Long,
}

impl PhrFormat {
    pub const fn max_psdu_length(self) -> u16 {
        match self {
            PhrFormat::Standard => 127,
            PhrFormat::Long => 1023,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PanId(u16);

impl PanId {
    pub const fn new(addr: u16) -> Self {
        Self(addr)
    }

    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ShortAddress(u16);

impl ShortAddress {
    pub const fn new(addr: u16) -> Self {
        Self(addr)
    }

    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExtendedAddress(u64);

impl ExtendedAddress {
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    /// Low power consumption, time is not kept
    Reset,
    /// Moderate power consumption, precision time is kept
    Idle,
}

// TODO: add XTAL trim option
// TODO: add frame format (127 vs 1023 octets)
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AutoAckConfig {
    pub preamble_length: PreambleLength,
    // add AIFS duration
}

impl Default for AutoAckConfig {
    fn default() -> Self {
        Self {
            preamble_length: PreambleLength::Symbols64,
        }
    }
}

// TODO: add XTAL trim option
// TODO: add frame format (127 vs 1023 octets)
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    pub preamble_code: PreambleCode,
    pub sfd_type: SfdType,
    pub phr_format: PhrFormat,
    pub rx_preamble_length_max: PreambleLength,
    pub preamble_timeout: Option<time::Duration>,
    /// Replace last FCS_LENGH octets with calculated FCS
    pub correct_tx_fcs: bool,

    pub auto_ack: Option<AutoAckConfig>,
    // TODO: add phr_data_rate, a.k.a. phyHrpUwbPhrDataRate,
}

impl Config {
    // TODO: make presets for IEEE operation parameter sets, see 15.7
    pub const fn new() -> Self {
        Self {
            preamble_code: PreambleCode::Code9,
            sfd_type: SfdType::Sfd0,
            phr_format: PhrFormat::Standard,
            rx_preamble_length_max: PreambleLength::Symbols4096,
            preamble_timeout: None,
            correct_tx_fcs: false,
            auto_ack: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct FrameTypeFilter: u8 {
        const BEACON = 1 << 0;
        const DATA = 1 << 1;
        const ACK = 1 << 2;
        const MAC_COMMAND = 1 << 3;
        const _RESERVED100 = 1 << 4;
        const MULTIPURPOSE = 1 << 5;
        const FRAGMENT = 1 << 6;
        const EXTENDED = 1 << 7;
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for FrameTypeFilter {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "FrameTypeFilter({=u8})", self.bits());
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FrameFilter {
    pub frame_type_filter: FrameTypeFilter,
    /// Accept some frames with only source address
    ///
    /// Conditions:
    /// * MAC command, data frames: source PAN ID should match own PAN ID
    /// * Multipurpose frames: destination PAN ID should match own PAN ID
    pub to_pan_coordinator: bool,
    /// Allow implicit broadcast
    ///
    /// Frame without destination PAN ID and destination address are treated as
    /// address to the broadcast PAN ID (0xffff) and broadcast address (0xffff)
    pub implicit_broadcast: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TxConfig {
    pub preamble_length: PreambleLength,
    pub bit_rate: BitRate,
    pub phr_ranging_flag: bool,
}

impl Default for TxConfig {
    fn default() -> Self {
        Self {
            preamble_length: PreambleLength::Symbols64,
            bit_rate: BitRate::Kbs850,
            phr_ranging_flag: false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RxReport<T> {
    pub ranging_flag: bool,
    pub length: u16,
    pub bit_rate: BitRate,
    pub fcs_good: bool,
    pub timestamp: T,
    pub imm_ack: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OpError {
    ProhibitedInCurrentState,
    IncompatibleCode,
    StartInstantPassed,
    PreambleTimeout,
    FrameTimeout,
    RxUnderflow,
    RxOverflow,
    BufferAccessBeyondPhrFormat(usize, PhrFormat),
    TxLengthAbovePhrFormat(u16, PhrFormat),
    TxLengthLessThanFcs(u16),
}

#[derive(Debug)]
pub enum Error<IF, DEV> {
    Interface(IF),
    Device(DEV),
    Operation(OpError),
}

#[cfg(feature = "defmt")]
impl<IF: defmt::Format, DEV: defmt::Format> defmt::Format for Error<IF, DEV> {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::Interface(iface) => defmt::write!(f, "Error::Interface({:?})", iface),
            Error::Device(dev) => defmt::write!(f, "Error::Device({:?})", dev),
            Error::Operation(op) => defmt::write!(f, "Error::Operation({:?})", op),
        }
    }
}

impl<IF, DEV> From<OpError> for Error<IF, DEV> {
    fn from(value: OpError) -> Self {
        Error::Operation(value)
    }
}

// Basic phy service with exclusive async interface.
//
// All operation should run to completion
#[allow(async_fn_in_trait)]
pub trait Phy {
    type Instant: time::CyclicTimestamp;
    type IoError: core::fmt::Debug;
    type DevError: core::fmt::Debug;

    const MAX_RX_PREAMBLE_TIMEOUT: time::Duration;
    const MAX_RX_FRAME_TIMEOUT: time::Duration;

    fn state(&self) -> State;
    async fn reset(&mut self) -> Result<(), Error<Self::IoError, Self::DevError>>;
    async fn start(&mut self, config: Config) -> Result<(), Error<Self::IoError, Self::DevError>>;

    async fn get_extended_address(
        &mut self,
    ) -> Result<ExtendedAddress, Error<Self::IoError, Self::DevError>>;

    async fn set_extended_address(
        &mut self,
        value: ExtendedAddress,
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    async fn get_pan_address(
        &mut self,
    ) -> Result<(PanId, ShortAddress), Error<Self::IoError, Self::DevError>>;

    async fn set_pan_address(
        &mut self,
        pan_id: PanId,
        short_addr: ShortAddress,
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    async fn set_frame_filter(
        &mut self,
        value: Option<FrameFilter>,
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    async fn get_timestamp(
        &mut self,
    ) -> Result<Self::Instant, Error<Self::IoError, Self::DevError>>;

    async fn write_tx_buffer(
        &mut self,
        psdu: &[u8],
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    async fn read_rx_buffer(
        &mut self,
        psdu: &mut [u8],
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    // Frame length should include FCS field
    async fn transmit(
        &mut self,
        config: TxConfig,
        length: u16,
        start_at: Self::Instant,
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    async fn transmit_w4r(
        &mut self,
        config: TxConfig,
        length: u16,
        start_at: Self::Instant,
        rx_timeout: time::Duration,
    ) -> Result<Option<RxReport<Self::Instant>>, Error<Self::IoError, Self::DevError>>;

    async fn receive(
        &mut self,
        start_at: Self::Instant,
        rx_timeout: time::Duration,
    ) -> Result<Option<RxReport<Self::Instant>>, Error<Self::IoError, Self::DevError>>;
}

// See IEEE 802.15.4-2020, Table 15-5
pub const fn preamble_symbol_duration(prf: Prf) -> time::Duration {
    match prf {
        Prf::Mhz16 => time::Duration::CHIP.mul_u32(496),
        Prf::Mhz64 => time::Duration::CHIP.mul_u32(508),
    }
}

pub const fn psdu_symbol_duration(bit_rate: BitRate) -> time::Duration {
    // See IEEE802.15.4-2020 table 15.3
    match bit_rate {
        BitRate::Kbs850 => time::Duration::CHIP.mul_u32(512),
        BitRate::Kbs6810 => time::Duration::CHIP.mul_u32(64),
    }
}

pub const fn shr_duration(
    prf: Prf,
    sfd_type: SfdType,
    preamble_length: PreambleLength,
) -> time::Duration {
    let preamble_symbol_duration = preamble_symbol_duration(prf);
    let sync_length = preamble_length.as_symbols();
    let sfd_length = sfd_type.symbol_length();
    let shr_length = sync_length + sfd_length as u16;
    preamble_symbol_duration.mul_u32(shr_length as u32)
}

pub const fn phr_duration(bit_rate: BitRate) -> time::Duration {
    const PHR_BITS: u32 = 18;
    psdu_symbol_duration(bit_rate).mul_u32(PHR_BITS)
}

pub const fn psdu_duration(bit_rate: BitRate, length: u16) -> time::Duration {
    const RS_IN_BLOCK_SIZE: u32 = 330;
    const RS_PARITY_SIZE: u32 = 48;
    let in_bit_length = length as u32 * 8;
    let block_count = in_bit_length.div_ceil(RS_IN_BLOCK_SIZE);
    let out_bit_length = in_bit_length + block_count * RS_PARITY_SIZE;
    psdu_symbol_duration(bit_rate).mul_u32(out_bit_length)
}
