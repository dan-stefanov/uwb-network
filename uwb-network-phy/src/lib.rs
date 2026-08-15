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

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SfdType {
    /// 0+0-+00-, IEEE 802.15.4 short 8-symbol SFD
    Sfd0,
    /// --+--, IEEE 802.15.4z short 4-symbol SFD
    Sfd1,
    /// ---+--+-, IEEE 802.15.4z defined 8-symbol SFD
    Sfd2,
    /// -----++--+--+--+-, IEEE 802.15.4z defined 16-symbol SFD
    Sfd3,
    /// -------+--+--+-+--+---++---+--++--, IEEE 802.15.4z defined 32-symbol SFD
    Sfd4,
}

impl SfdType {
    pub const fn as_symbols(self) -> u16 {
        match self {
            Self::Sfd1 => 4,
            Self::Sfd0 | Self::Sfd2 => 8,
            Self::Sfd3 => 16,
            Self::Sfd4 => 32,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BitRate {
    /// 0.11 MBit/s for PHR and payload
    Kbs110,
    /// 0.85 MBit/s for PHR and payload
    Kbs850,
    /// 0.85 MBit/s for PHR, 6.81 MBit/s for payload, a.k.a. DRBM_LP
    Kbs6810,
    /// 6.81 MBit/s for PHR and payload, a.k.a. DRBM_HP
    Kbs6810Only,
    /// 0.85 MBit/s for PHR, 27.24 MBit/s for payload
    Kbs27240,
}

pub const MAX_PSDU_LENGTH: u16 = 127;
pub const MAX_LONG_PSDU_LENGTH: u16 = 1023;

bitflags! {
    #[cfg_attr(not(feature = "defmt"), derive(Clone, Copy, Eq, PartialEq, Debug))]
    pub struct Capabilities: u64 {
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

        const PRF_4 = 1 << 16;
        const PRF_16 = 1 << 17;
        const PRF_62 = 1 << 18;
        const PRF_111 = 1 << 19;

        const PSR_16 = 1 << 20;
        const PSR_32 = 1 << 21;
        const PSR_64 = 1 << 22;
        const PSR_128 = 1 << 23;
        const PSR_256 = 1 << 24;
        const PSR_512 = 1 << 25;
        const PSR_1024 = 1 << 26;
        const PSR_1536 = 1 << 27;
        const PSR_2048 = 1 << 28;
        const PSR_4096 = 1 << 29;

        const SFD_0 = 1 << 30;
        const SFD_1 = 1 << 31;
        const SFD_2 = 1 << 32;
        const SFD_3 = 1 << 33;
        const SFD_4 = 1 << 34;

        const BIT_RATE_110 = 1 << 35;
        const BIT_RATE_850 = 1 << 36;
        const BIT_RATE_6810 = 1 << 37;
        const BIT_RATE_6810_ONLY = 1 << 38;
        const BIT_RATE_27240 = 1 << 39;

        const LONG_FRAME_FORMAT = 1 << 40;
        const CORRECT_TX_FCS = 1 << 41;
    }
}

impl Capabilities {
    pub const fn has_channel(self, channel: Channel) -> bool {
        self.contains(Self::from_channel(channel))
    }

    pub const fn has_prf(self, prf: MeanPrf) -> bool {
        self.contains(Self::from_prf(prf))
    }

    pub const fn has_psr(self, psr: Psr) -> bool {
        self.contains(Self::from_psr(psr))
    }

    pub const fn has_sfd(self, sfd_type: SfdType) -> bool {
        self.contains(Self::from_sfd(sfd_type))
    }

    pub const fn has_bit_rate(self, bit_rate: BitRate) -> bool {
        self.contains(Self::from_bit_rate(bit_rate))
    }

    pub const fn has_long_frame_format(self) -> bool {
        self.contains(Self::LONG_FRAME_FORMAT)
    }

    pub const fn has_correct_tx_fcs(self) -> bool {
        self.contains(Self::CORRECT_TX_FCS)
    }

    const fn from_channel(channel: Channel) -> Self {
        Self::from_bits_truncate(1 << channel.as_number())
    }

    const fn from_prf(prf: MeanPrf) -> Self {
        match prf {
            MeanPrf::Mhz4 => Self::PRF_4,
            MeanPrf::Mhz16 => Self::PRF_16,
            MeanPrf::Mhz62 => Self::PRF_62,
            MeanPrf::Mhz111 => Self::PRF_111,
        }
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

    const fn from_sfd(sfd_type: SfdType) -> Self {
        match sfd_type {
            SfdType::Sfd0 => Self::SFD_0,
            SfdType::Sfd1 => Self::SFD_1,
            SfdType::Sfd2 => Self::SFD_2,
            SfdType::Sfd3 => Self::SFD_3,
            SfdType::Sfd4 => Self::SFD_4,
        }
    }

    const fn from_bit_rate(bit_rate: BitRate) -> Self {
        match bit_rate {
            BitRate::Kbs110 => Self::BIT_RATE_110,
            BitRate::Kbs850 => Self::BIT_RATE_850,
            BitRate::Kbs6810 => Self::BIT_RATE_6810,
            BitRate::Kbs6810Only => Self::BIT_RATE_6810_ONLY,
            BitRate::Kbs27240 => Self::BIT_RATE_27240,
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

#[non_exhaustive]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RunConfig {
    pub channel: Channel,
    pub prf: MeanPrf,
    pub rx_preamble_code: u8,
    pub tx_preamble_code: u8,
    pub psr: Psr,
    pub sfd_type: SfdType,
    pub bit_rate: BitRate,
    pub long_frame_format: bool,
    pub ranging: bool,
    /// Replace last FCS_LENGTH octets with calculated FCS
    pub correct_tx_fcs: bool,
}

impl RunConfig {
    pub const fn new() -> Self {
        Self {
            channel: Channel::CH_9,
            prf: MeanPrf::Mhz62,
            rx_preamble_code: 9,
            tx_preamble_code: 9,
            psr: Psr::Symbols64,
            sfd_type: SfdType::Sfd0,
            bit_rate: BitRate::Kbs850,
            long_frame_format: false,
            ranging: false,
            correct_tx_fcs: false,
        }
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConfigError {
    UnsupportedChannel(Channel),
    UnsupportedPrf(MeanPrf),
    InvalidRxPreambleCode(u8, MeanPrf),
    InvalidTxPreambleCode(u8, MeanPrf),
    UnsupportedPsr(Psr),
    UnsupportedSfd(SfdType),
    UnsupportedBitRate(BitRate),
    UnsupportedLongFrameFormat,
    UnsupportedCorrectTxFcs,
}

impl RunConfig {
    pub fn check_capabilities(&self, capabilities: Capabilities) -> Result<(), ConfigError> {
        if !capabilities.has_channel(self.channel) {
            return Err(ConfigError::UnsupportedChannel(self.channel));
        }
        if !capabilities.has_psr(self.psr) {
            return Err(ConfigError::UnsupportedPsr(self.psr));
        }

        if !capabilities.has_prf(self.prf) {
            return Err(ConfigError::UnsupportedPrf(self.prf));
        }
        let [min_code, max_code] = self.prf.code_range();
        if !(min_code <= self.rx_preamble_code && self.rx_preamble_code <= max_code) {
            return Err(ConfigError::InvalidRxPreambleCode(
                self.rx_preamble_code,
                self.prf,
            ));
        }
        if !(min_code <= self.tx_preamble_code && self.tx_preamble_code <= max_code) {
            return Err(ConfigError::InvalidTxPreambleCode(
                self.tx_preamble_code,
                self.prf,
            ));
        }
        if !capabilities.has_sfd(self.sfd_type) {
            return Err(ConfigError::UnsupportedSfd(self.sfd_type));
        }
        if !capabilities.has_bit_rate(self.bit_rate) {
            return Err(ConfigError::UnsupportedBitRate(self.bit_rate));
        }
        if self.long_frame_format && !capabilities.has_long_frame_format() {
            return Err(ConfigError::UnsupportedLongFrameFormat);
        }
        if self.correct_tx_fcs && !capabilities.has_correct_tx_fcs() {
            return Err(ConfigError::UnsupportedCorrectTxFcs);
        }

        Ok(())
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
    UnsupportedConfig(ConfigError),
    ExcessiveRxTimeout(time::Duration),
    StartInstantPassed(time::Duration),
    BufferAccessBeyondFrameFormat(usize, u16),
    TxLengthAboveFrameFormat(u16, u16),
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

    fn state(&self) -> State;
    fn capabilities(&self) -> Capabilities;
    fn max_rx_timeout(&self) -> time::Duration;
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

    /// Transmit a frame form buffer
    ///
    /// `psdu_length` should include FCS field
    async fn transmit(
        &mut self,
        start_at: Self::Instant,
        psdu_length: u16,
    ) -> Result<(), Error<Self::IoError, Self::DevError>>;

    /// Try to receive a frame
    ///
    /// `max_preamble_hunt` is the minimum preamble hunting duration in symbols. Once it
    /// expires, the implementation may stop reception to save power.
    /// `timeout` define the total time to the frame reception. Once timeout is expired,
    /// implementation should return, event if a frame is being receiving.
    /// The value should not exceed the value returned by [`Phy::max_rx_timeout`].
    async fn receive(
        &mut self,
        start_at: Self::Instant,
        max_preamble_hunt: Option<NonZeroU16>,
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

pub const fn phr_bit_duration(bit_rate: BitRate) -> time::Duration {
    // See IEEE802.15.4-2020 table 15.3
    match bit_rate {
        BitRate::Kbs110 => time::Duration::CHIP.mul_u32(4096),
        BitRate::Kbs850 | BitRate::Kbs6810 | BitRate::Kbs27240 => time::Duration::CHIP.mul_u32(512),
        BitRate::Kbs6810Only => time::Duration::CHIP.mul_u32(64),
    }
}

pub const fn psdu_bit_duration(bit_rate: BitRate) -> time::Duration {
    // See IEEE802.15.4-2020 table 15.3
    match bit_rate {
        BitRate::Kbs110 => time::Duration::CHIP.mul_u32(4096),
        BitRate::Kbs850 => time::Duration::CHIP.mul_u32(512),
        BitRate::Kbs6810 | BitRate::Kbs6810Only => time::Duration::CHIP.mul_u32(64),
        BitRate::Kbs27240 => time::Duration::CHIP.mul_u32(16),
    }
}

pub const fn shr_duration(prf: MeanPrf, sfd_type: SfdType, psr: Psr) -> time::Duration {
    let preamble_symbol_duration = preamble_symbol_duration(prf);
    let sync_length = psr.as_symbols();
    let shr_length = sync_length + sfd_type.as_symbols();
    preamble_symbol_duration.mul_u32(shr_length as u32)
}

pub const fn phr_duration(bit_rate: BitRate) -> time::Duration {
    const PHR_BITS: u32 = 18;
    phr_bit_duration(bit_rate).mul_u32(PHR_BITS)
}

pub const fn psdu_duration(bit_rate: BitRate, length: u16) -> time::Duration {
    const RS_IN_BLOCK_SIZE: u32 = 330;
    const RS_PARITY_SIZE: u32 = 48;
    let in_bit_length = length as u32 * 8;
    let block_count = in_bit_length.div_ceil(RS_IN_BLOCK_SIZE);
    let out_bit_length = in_bit_length + block_count * RS_PARITY_SIZE;
    psdu_bit_duration(bit_rate).mul_u32(out_bit_length)
}
