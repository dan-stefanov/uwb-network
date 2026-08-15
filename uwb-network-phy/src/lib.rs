#![cfg_attr(not(test), no_std)]

use core::num::NonZeroU16;

#[cfg(not(feature = "defmt"))]
use bitflags::bitflags;
#[cfg(feature = "defmt")]
use defmt::bitflags;

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod time;

#[cfg(feature = "functional_tests")]
pub mod functional_tests;

pub const FCS_LENGTH: u16 = 2;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Channel(u8);

impl Channel {
    /// Center 499.2 MHz, Bandwidth 499.2 MHz
    pub const CH_0: Self = Self(0);
    /// Center 3494.4 MHz, Bandwidth 499.2 MHz
    pub const CH_1: Self = Self(1);
    /// Center 3993.6 MHz, Bandwidth 499.2 MHz
    pub const CH_2: Self = Self(2);
    /// Center 4492.8 MHz, Bandwidth 499.2 MHz
    pub const CH_3: Self = Self(3);
    /// Center 3993.6 MHz, Bandwidth 1331.2 MHz
    pub const CH_4: Self = Self(4);
    /// Center 6489.6 MHz, Bandwidth 499.2 MHz
    pub const CH_5: Self = Self(5);
    /// Center 6988.8 MHz, Bandwidth 499.2 MHz
    pub const CH_6: Self = Self(6);
    /// Center 6489.6 MHz, Bandwidth 1081.6 MHz
    pub const CH_7: Self = Self(7);
    /// Center 7488.0 MHz, Bandwidth 499.2 MHz
    pub const CH_8: Self = Self(8);
    /// Center 7987.2 MHz, Bandwidth 499.2 MHz
    pub const CH_9: Self = Self(9);
    /// Center 8486.4 MHz, Bandwidth 499.2 MHz
    pub const CH_10: Self = Self(10);
    /// Center 7987.2 MHz, Bandwidth 1331.2 MHz
    pub const CH_11: Self = Self(11);
    /// Center 8985.6 MHz, Bandwidth 499.2 MHz
    pub const CH_12: Self = Self(12);
    /// Center 9484.8 MHz, Bandwidth 499.2 MHz
    pub const CH_13: Self = Self(13);
    /// Center 9984.0 MHz, Bandwidth 499.2 MHz
    pub const CH_14: Self = Self(14);
    /// Center 9484.8 MHz, Bandwidth 1354.97 MHz
    pub const CH_15: Self = Self(15);

    pub const fn new_truncating(value: u8) -> Self {
        Self(value & 0xf)
    }

    pub const fn as_number(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MeanPrf {
    /// 31-symbols preambles, CHIP_FREQ / 64
    Mhz4,
    /// 31-symbols preambles, CHIP_FREQ / 16
    Mhz16,
    /// 127-symbols preambles, CHIP_FREQ / 4
    Mhz62,
    /// 91-symbols preambles, CHIP_FREQ / 4, IEEE 802.15.4z
    Mhz111,
}

impl MeanPrf {
    pub const fn code_range(self) -> [u8; 2] {
        match self {
            MeanPrf::Mhz4 | MeanPrf::Mhz16 => [1, 8],
            MeanPrf::Mhz62 => [9, 24],
            MeanPrf::Mhz111 => [25, 32],
        }
    }
}

pub const fn ieee_allocated_code_range(chan: Channel, prf: MeanPrf) -> [u8; 2] {
    match (prf, chan.as_number()) {
        (MeanPrf::Mhz4 | MeanPrf::Mhz16, 0 | 1 | 8 | 12) => [1, 2],
        (MeanPrf::Mhz4 | MeanPrf::Mhz16, 2 | 5 | 9 | 13) => [3, 4],
        (MeanPrf::Mhz4 | MeanPrf::Mhz16, 3 | 6 | 10 | 14) => [5, 6],
        (MeanPrf::Mhz4 | MeanPrf::Mhz16, 4 | 7 | 11 | 15) => [7, 8],
        (MeanPrf::Mhz62, 0..=3 | 5 | 6 | 8..=10 | 12..=14) => [9, 12],
        (MeanPrf::Mhz62, 4 | 7 | 11 | 15) => [17, 20],
        (MeanPrf::Mhz111, 0..=15) => [25, 32],
        (_, 16..) => core::unreachable!(),
    }
}

/// Preamble symbol repetitions
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Psr {
    /// IEEE 802.15.4 standard length
    Symbols16,
    /// Non-standard length
    Symbols32,
    /// IEEE 802.15.4, IEEE 802.15.8 standard length
    Symbols64,
    /// IEEE 802.15.8 standard length
    Symbols128,
    /// IEEE 802.15.8 standard length
    Symbols256,
    /// IEEE 802.15.8 standard length
    Symbols512,
    /// IEEE 802.15.4, IEEE 802.15.8 standard length
    Symbols1024,
    /// IEEE 802.15.8 standard length
    Symbols1536,
    /// IEEE 802.15.8 standard length
    Symbols2048,
    /// IEEE 802.15.4, IEEE 802.15.8 standard length
    Symbols4096,
}

impl Psr {
    pub const fn as_symbols(self) -> u16 {
        match self {
            Psr::Symbols16 => 16,
            Psr::Symbols32 => 32,
            Psr::Symbols64 => 64,
            Psr::Symbols128 => 128,
            Psr::Symbols256 => 256,
            Psr::Symbols512 => 512,
            Psr::Symbols1024 => 1024,
            Psr::Symbols1536 => 1536,
            Psr::Symbols2048 => 2048,
            Psr::Symbols4096 => 4096,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SfdLength {
    Symbols4 = 4,
    Symbols8 = 8,
    Symbols16 = 16,
    Symbols32 = 32,
    Symbols64 = 64,
}

impl From<SfdLength> for u8 {
    fn from(value: SfdLength) -> Self {
        value as Self
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

bitflags! {
    #[cfg_attr(not(feature = "defmt"), derive(Clone, Copy, Eq, PartialEq, Debug))]
    pub struct Capabilities: u32 {
        const CH_0 = 1 << Channel::CH_0.as_number();
        const CH_1 = 1 << Channel::CH_1.as_number();
        const CH_2 = 1 << Channel::CH_2.as_number();
        const CH_3 = 1 << Channel::CH_3.as_number();
        const CH_4 = 1 << Channel::CH_4.as_number();
        const CH_5 = 1 << Channel::CH_5.as_number();
        const CH_6 = 1 << Channel::CH_6.as_number();
        const CH_7 = 1 << Channel::CH_7.as_number();
        const CH_8 = 1 << Channel::CH_8.as_number();
        const CH_9 = 1 << Channel::CH_9.as_number();
        const CH_10 = 1 << Channel::CH_10.as_number();
        const CH_11 = 1 << Channel::CH_11.as_number();
        const CH_12 = 1 << Channel::CH_12.as_number();
        const CH_13 = 1 << Channel::CH_13.as_number();
        const CH_14 = 1 << Channel::CH_14.as_number();
        const CH_15 = 1 << Channel::CH_15.as_number();

        const PSR_16 = 1 << 16;
        const PSR_32 = 1 << 17;
        const PSR_64 = 1 << 18;
        const PSR_128 = 1 << 19;
        const PSR_256 = 1 << 20;
        const PSR_512 = 1 << 21;
        const PSR_1024 = 1 << 22;
        const PSR_1536 = 1 << 23;
        const PSR_2048 = 1 << 24;
        const PSR_4096 = 1 << 25;
    }
}

impl Capabilities {
    pub fn has_channel(self, channel: Channel) -> bool {
        self.contains(Self::from_channel(channel))
    }

    pub fn has_psr(self, psr: Psr) -> bool {
        self.contains(Self::from_psr(psr))
    }

    const fn from_channel(channel: Channel) -> Self {
        Self::from_bits_truncate(1 << channel.as_number())
    }

    const fn from_psr(psr: Psr) -> Self {
        match psr {
            Psr::Symbols16 => Self::PSR_16,
            Psr::Symbols32 => Self::PSR_32,
            Psr::Symbols64 => Self::PSR_64,
            Psr::Symbols128 => Self::PSR_128,
            Psr::Symbols256 => Self::PSR_256,
            Psr::Symbols512 => Self::PSR_512,
            Psr::Symbols1024 => Self::PSR_1024,
            Psr::Symbols1536 => Self::PSR_1536,
            Psr::Symbols2048 => Self::PSR_2048,
            Psr::Symbols4096 => Self::PSR_4096,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    /// Low power consumption, time is not kept
    Stopped,
    /// Moderate power consumption, precision time is kept
    Running,
}

// TODO: add XTAL trim option
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RunConfig {
    pub preamble_code: u8,
    pub psr: Psr,
    pub phr_format: PhrFormat,
    /// Match PHR bit rate to the PSDU bit rate
    ///
    /// When cleared, use BitRate::Kbs6810 for any PSDU bit rate
    pub high_phr_bit_rate: bool,
    /// Replace last FCS_LENGTH octets with calculated FCS
    pub correct_tx_fcs: bool,
}

// Preamble code depends on chosen PRF
// TODO: remove default
impl Default for RunConfig {
    fn default() -> Self {
        Self {
            preamble_code: 0,
            psr: Psr::Symbols64,
            phr_format: PhrFormat::Standard,
            high_phr_bit_rate: false,
            correct_tx_fcs: false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TxConfig {
    pub bit_rate: BitRate,
    pub ranging_flag: bool,
}

impl Default for TxConfig {
    fn default() -> Self {
        Self {
            bit_rate: BitRate::Kbs850,
            ranging_flag: false,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RxConfig {
    /// Preamble hunting duration in symbols
    ///
    /// Implementation should hunt for preamble at least for this duration.
    /// Once timeout has expired, implementation may stop reception to
    /// save power.
    pub max_preamble_hunt: Option<NonZeroU16>,
}

impl Default for RxConfig {
    fn default() -> Self {
        Self {
            max_preamble_hunt: None,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RxReport<T> {
    pub length: u16,
    pub fcs_good: bool,
    pub timestamp: T,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OpError {
    ProhibitedInCurrentState(State),
    IncompatiblePreambleCode(u8, MeanPrf),
    UnsupportedPsr(Psr),
    ExcessiveRxTimeout(time::Duration),
    StartInstantPassed(time::Duration),
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
            Error::Interface(interface) => defmt::write!(f, "Error::Interface({:?})", interface),
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

    const MAX_RX_FRAME_TIMEOUT: time::Duration;

    fn state(&self) -> State;
    fn capabilities(&self) -> Capabilities;
    fn channel(&self) -> Channel;
    fn preamble_prf(&self) -> MeanPrf;
    fn sfd_length(&self) -> SfdLength;
    async fn stop(&mut self) -> Result<(), Error<Self::IoError, Self::DevError>>;
    async fn start(
        &mut self,
        config: RunConfig,
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

    async fn receive(
        &mut self,
        config: RxConfig,
        start_at: Self::Instant,
        timeout: time::Duration,
    ) -> Result<Option<RxReport<Self::Instant>>, Error<Self::IoError, Self::DevError>>;
}

// See IEEE 802.15.4-2020, Table 15-5
pub const fn preamble_symbol_duration(prf: MeanPrf) -> time::Duration {
    match prf {
        MeanPrf::Mhz4 => time::Duration::CHIP.mul_u32(31 * 64),
        MeanPrf::Mhz16 => time::Duration::CHIP.mul_u32(31 * 16),
        MeanPrf::Mhz62 => time::Duration::CHIP.mul_u32(127 * 4),
        MeanPrf::Mhz111 => time::Duration::CHIP.mul_u32(91 * 4),
    }
}

pub const fn psdu_symbol_duration(bit_rate: BitRate) -> time::Duration {
    // See IEEE802.15.4-2020 table 15.3
    match bit_rate {
        BitRate::Kbs850 => time::Duration::CHIP.mul_u32(512),
        BitRate::Kbs6810 => time::Duration::CHIP.mul_u32(64),
    }
}

pub const fn shr_duration(prf: MeanPrf, sfd_length: SfdLength, psr: Psr) -> time::Duration {
    let preamble_symbol_duration = preamble_symbol_duration(prf);
    let sync_length = psr.as_symbols();
    let shr_length = sync_length + sfd_length as u8 as u16;
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
