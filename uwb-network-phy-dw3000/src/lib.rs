#![no_std]

use core::num::NonZeroU16;
use interface::Interface;
use ral::regs::{DgcCfgLutData, DgcLutData, EventsLow as Events};
use ral::{RegisterAccess, regs};
use uwb_network_phy::{self as phy, Error, OpError, time::CyclicTimestamp, time::Duration};

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod interface;
mod otp;

#[allow(dead_code)]
mod ral;

// 124.8 MHZ system clock period
const SYSTEM_TIME_UNIT: Duration = Duration::CHIP.mul_u32(4);
// 31-bit system clock counter
const SYSTEM_TIME_PERIOD: Duration = SYSTEM_TIME_UNIT.mul_u32(1u32 << 31);

type Instant = phy::time::Instant<{ SYSTEM_TIME_PERIOD.as_ticks() }>;

// The following duration are measured on host side, but we use the same
// Duration type for simplicity
const RESET_DELAY_US: u32 = 1_000;
// initialization used to last 1200us
const XTAL_WAKE_UP_TIMEOUT_US: u32 = 2_000;
const PLL_LOCK_TIMEOUT_US: u32 = 150_000;
const RX_CALIBRATION_TIMEOUT_US: u32 = 60_000;
const RX_CALIBRATION_POLL_PERIOD_US: u32 = 20_000;

const RX_FRAME_TIMEOUT_UNIT: Duration = Duration::CHIP.mul_u32(512);
const MAX_RX_FRAME_TIMEOUT: Duration = RX_FRAME_TIMEOUT_UNIT.mul_u32((1u32 << 20) - 1);

// minimum microsecond duration in host system relative to DW3000 clock
const HOST_MICROSECOND_MIN: Duration = {
    const CLOCK_TOL: f32 = 0.05; // STM32 HSI16 are typically below 2%
    let ticks = (Duration::SECOND.as_ticks() as f32 / 1.0e6 / (1.0 + CLOCK_TOL)) as u64;
    core::assert!(ticks > 0);
    Duration::from_ticks(ticks)
};

/// QORVO Register Identification Tag
const DEV_RIDTAG: u16 = 0xdeca;
/// DW3000 device
const DEV_MODEL: u8 = 0x03;
/// Non-PDoA device
const DEV_VERSION: u8 = 0x0;
const DEV_REVISION: u8 = 0x2;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq)]
enum FastCommand {
    /// Puts the device into IDLE state and clears any events
    Txrxoff = 0x00,
    /// Immediate start of transmission
    Tx = 0x01,
    /// Enable RX immediately
    Rx = 0x02,
    /// Delayed TX w.r.t. DX_TIME
    Dtx = 0x03,
    /// Delayed RX w.r.t. DX_TIME
    Drx = 0x04,
    /// Delayed TX w.r.t. TX timestamp + DX_TIME
    DtxTs = 0x05,
    /// Delayed RX w.r.t. TX timestamp + DX_TIME
    DrxTs = 0x06,
    /// Delayed TX w.r.t. RX timestamp + DX_TIME
    DtxRs = 0x07,
    /// Delayed RX w.r.t. RX timestamp + DX_TIME
    DrxRs = 0x08,
    /// Delayed TX w.r.t. DREF_TIME + DX_TIME
    DtxRef = 0x09,
    /// Delayed RX w.r.t. DREF_TIME + DX_TIME
    DrxRef = 0x0a,
    /// TX if no preamble detected
    CcaTx = 0x0b,
    /// Start TX immediately, then when TX is done, enable the receiver
    TxW4r = 0x0c,
    /// Delayed TX w.r.t. DX_TIME, then enable receiver
    DtxW4r = 0x0d,
    /// Delayed TX w.r.t. TX timestamp + DX_TIME, then enable receiver
    DtxTsW4r = 0x0e,
    /// Delayed TX w.r.t. RX timestamp + DX_TIME, then enable receiver
    DtxRsW4r = 0x0f,
    /// Delayed TX w.r.t. DREF_TIME + DX_TIME, then enable receiver
    DtxRefW4r = 0x10,
    /// TX packet if no preamble detected, then enable receive
    CcaTxW4r = 0x11,
    /// Clear all interrupt events
    ClrIrqs = 0x12,
    /// Toggle double buffer pointer
    DbToggle = 0x13,
}

#[rustfmt::skip]
static RX_TUNE_DGC_CFG: DgcCfgLutData = [
    0x40, 0x02, 0x00, 0x10,
    0x89, 0xa4, 0x6d, 0x1b,
    0x23, 0xc9, 0xb6, 0x2d,
    0xb5, 0x6d, 0x20, 0x12,
    0xda, 0xb6, 0x91, 0x24,
    0x24, 0x49, 0xb6, 0x2d,
    0x6d, 0xdb, 0x16, 0x00,
];

#[rustfmt::skip]
static RX_TUNE_DGC_LUT_CH5: DgcLutData = [
    0xfd, 0xc0, 0x01, 0x00,
    0x3e, 0xc4, 0x01, 0x00,
    0xbe, 0xc6, 0x01, 0x00,
    0x7e, 0xc7, 0x01, 0x00,
    0x36, 0xcf, 0x01, 0x00,
    0xb5, 0xcf, 0x01, 0x00,
    0xf5, 0xcf, 0x01, 0x00,
];

#[rustfmt::skip]
static RX_TUNE_DGC_LUT_CH9: DgcLutData = [
    0xfe, 0xa8, 0x02, 0x00,
    0x36, 0xac, 0x02, 0x00,
    0xfe, 0xa5, 0x02, 0x00,
    0x3e, 0xaf, 0x02, 0x00,
    0x7d, 0xaf, 0x02, 0x00,
    0xb5, 0xaf, 0x02, 0x00,
    0xb5, 0xaf, 0x02, 0x00,
];

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DeviceError {
    ResetTimeout,
    SpiNotReady,
    WrongDevice,
    PllLockTimeout,
    RxCalibrationTimeout,
    RxCalibrationFailure,
    TxStateTimeout { timeout_us: u32 },
    RxStateTimeout { timeout_us: u32 },
}

/// Subset of IEEE 802.15.4 HRP UWB channels supported by the DW3000.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Channel {
    /// 6489.6 MHz center, 499.2 MHz bandwidth
    Ch5,
    /// 7987.2 MHz center, 499.2 MHz bandwidth
    Ch9,
}

/// DW3000 receiver operating parameter set.
///
/// Check DW3000 User Manual section 8.2.12.7 for details.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RxOps {
    /// Parameter set 2, optimized for very short preambles, i.e. 64 symbols
    ShortPreamble,
    /// Parameter set 0, optimized for long preambles, i.e. 256 symbols or more
    LongPreamble,
}

/// DW3000 preamble acquisition chunk size.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PreambleAcquisitionChunk {
    /// For preamble length 32.
    Symbols4,
    /// For preamble length 64 or more.
    Symbols8,
    /// For preamble length 128 or more.
    Symbols16,
    /// For preamble length 256 or more (undocumented).
    Symbols32,
}

impl PreambleAcquisitionChunk {
    pub const MIN: PreambleAcquisitionChunk = PreambleAcquisitionChunk::Symbols4;

    pub const fn as_symbols(self) -> u8 {
        match self {
            Self::Symbols4 => 4,
            Self::Symbols8 => 8,
            Self::Symbols16 => 16,
            Self::Symbols32 => 32,
        }
    }
}

impl<IF: Interface> From<ral::Error<IF>> for Error<IF::Error, DeviceError> {
    fn from(value: ral::Error<IF>) -> Self {
        match value {
            ral::Error::Interface(err) => Error::Interface(err),
        }
    }
}

struct OtpReader<'a, IF> {
    interface: &'a mut IF,
}

impl<'a, IF: Interface> OtpReader<'a, IF> {
    fn new(interface: &'a mut IF) -> Self {
        Self { interface }
    }
}

impl<'a, IF: Interface> otp::OtpRead for OtpReader<'a, IF> {
    type Error = Error<IF::Error, DeviceError>;
    fn read_u32(&mut self, addr: u8) -> Result<u32, Error<IF::Error, DeviceError>> {
        let mut ral = RegisterAccess::new(self.interface);
        // set manual access mode
        ral.otp_cfg().write(|w| w.set_otp_man(true))?;
        // set the address
        ral.otp_addr().write(|w| w.set_otp_addr(addr as u16))?;
        // assert the read strobe
        ral.otp_cfg().write(|w| w.set_otp_read(true))?;
        // read the result
        let res = ral.otp_rdata().read_bytes()?;
        Ok(res)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct RfConfig {
    pub channel: Channel,
    pub prf: phy::Prf,
    pub pac: PreambleAcquisitionChunk,
    pub rx_ops: RxOps,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct ChannelConfig {
    pub channel: Channel,
    pub rx_code: phy::PreambleCode,
    pub tx_code: phy::PreambleCode,
    pub sfd_type: phy::SfdType,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
struct RxFrameInfo {
    pub frame_length: u16,
    pub bit_rate: phy::BitRate,
    pub ranging_flag: bool,
    /// Preamble length according to PHR, 127 octet frame only
    ///
    /// Valid values: 16, 64, 1024, 4096
    pub preamble_length_phr: Option<phy::PreambleLength>,
    /// Preamble length by accumulated symbols
    pub preamble_length_acc: phy::PreambleLength,
}

struct OtpData {
    ldotune_cal_set: bool,
    xtal_trim: u8,
    bias_tune_byte2: u8,
    rx_tune_set: bool,
    pll_lock_code: u32,
}

impl OtpData {
    fn load<R: otp::OtpRead>(mut otp: R) -> Result<Self, R::Error> {
        Ok(Self {
            ldotune_cal_set: otp.ldotune_cal()? != 0,
            xtal_trim: otp.xtal_trim()?,
            bias_tune_byte2: otp.bias_tune()?.to_le_bytes()[2],
            rx_tune_set: otp.rx_tune_dgc_cfg0()?.to_le_bytes() == RX_TUNE_DGC_CFG[0..4],
            pll_lock_code: otp.pll_lock_code()?,
        })
    }
}

const RX_TERMINATION_EVENTS: Events = Events::RXPHE
    .union(Events::RXFR)
    .union(Events::RXFSL)
    .union(Events::RXFTO)
    .union(Events::RXPTO)
    .union(Events::RXSTO)
    .union(Events::ARFE);

struct InterfaceWrapper<IF>(IF);

impl<IF: Interface> InterfaceWrapper<IF> {
    fn set_reset(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        self.0.set_reset().map_err(Error::Interface)
    }
    fn clear_reset(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        self.0.clear_reset().map_err(Error::Interface)
    }

    #[allow(dead_code)]
    fn wake_up(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        self.0.wake_up().map_err(Error::Interface)
    }

    fn send_command(&mut self, command: FastCommand) -> Result<(), Error<IF::Error, DeviceError>> {
        self.0.send_command(command as u8).map_err(Error::Interface)
    }

    fn ral(&mut self) -> RegisterAccess<'_, IF> {
        RegisterAccess::new(&mut self.0)
    }

    fn otp(&mut self) -> OtpReader<'_, IF> {
        OtpReader::new(&mut self.0)
    }

    #[allow(dead_code)]
    fn has_events(&mut self) -> Result<bool, Error<IF::Error, DeviceError>> {
        self.0.is_irq().map_err(Error::Interface)
    }

    async fn delay_us(&mut self, delay_us: u32) {
        self.0.delay_us(delay_us).await;
    }

    fn get_events(&mut self) -> Result<Events, Error<IF::Error, DeviceError>> {
        let mut ral = self.ral();
        let value = ral.sys_status_low().read_bytes()?;
        Ok(Events::from_bits_truncate(value))
    }

    fn clear_events(&mut self, events: Events) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.ral();
        ral.sys_status_low().clear_bytes(events.bits())?;
        Ok(())
    }

    fn clear_all_events(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        self.send_command(FastCommand::ClrIrqs)?;
        Ok(())
    }

    fn set_event_mask(&mut self, mask: Events) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.ral();
        ral.sys_enable_low().write_bytes(mask.bits())?;
        Ok(())
    }

    async fn wait_for_events(
        &mut self,
        timeout_us: u32,
    ) -> Result<bool, Error<IF::Error, DeviceError>> {
        self.0
            .wait_for_irq(timeout_us)
            .await
            .map_err(Error::Interface)
    }
}

async fn reset_power_up<IF: Interface>(
    interface: &mut InterfaceWrapper<IF>,
) -> Result<(), Error<IF::Error, DeviceError>> {
    interface.set_reset()?;
    interface.delay_us(RESET_DELAY_US).await;
    interface.clear_reset()?;

    // SPIRDY event is enabled by default
    if !interface.wait_for_events(XTAL_WAKE_UP_TIMEOUT_US).await? {
        return Err(Error::Device(DeviceError::ResetTimeout));
    }

    let events = interface.get_events()?;
    if !events.contains(Events::SPIRDY) {
        return Err(Error::Device(DeviceError::SpiNotReady));
    }
    Ok(())
}

async fn check_dev_id<IF: Interface>(
    interface: &mut InterfaceWrapper<IF>,
) -> Result<(), Error<IF::Error, DeviceError>> {
    let id = interface.ral().dev_id().read()?;
    if id.ridtag() != DEV_RIDTAG
        || id.model() != DEV_MODEL
        || id.ver() != DEV_VERSION
        || id.rev() != DEV_REVISION
    {
        return Err(Error::Device(DeviceError::WrongDevice));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum InnerState {
    Stopped,
    Running(phy::RunConfig),
}

impl From<InnerState> for phy::State {
    fn from(value: InnerState) -> Self {
        match value {
            InnerState::Stopped => phy::State::Stopped,
            InnerState::Running(_) => phy::State::Running,
        }
    }
}

pub struct Dw3000Phy<IF> {
    interface: InterfaceWrapper<IF>,
    otp: OtpData,
    dev_config: DeviceConfig,
    state: InnerState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
/// One TX_POWER gain setting.
///
/// The encoded byte uses bits 7:2 for the six-bit fine gain and bits 1:0 for
/// the two-bit coarse gain.
pub struct TxPower(u8);

impl TxPower {
    pub const fn new(coarse_gain: u8, fine_gain: u8) -> Self {
        core::assert!(coarse_gain < 4);
        core::assert!(fine_gain < 64);
        Self((fine_gain << 2) | coarse_gain)
    }
}

impl Default for TxPower {
    fn default() -> Self {
        // TX_POWER POR value, DW3000 UM 8.2.2.21.1.
        Self(0x82)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TxPowerConfig {
    /// Transmit power for the synchronization header.
    pub shr: TxPower,
    /// Transmit power for the PHY header.
    pub phr: TxPower,
    /// Transmit power for the PHY payload.
    pub data: TxPower,
    /// Transmit power for the Scrambled Timestamp Sequence.
    pub sts: TxPower,
}

impl TxPowerConfig {
    pub const fn new_uniform(power: TxPower) -> Self {
        Self {
            shr: power,
            phr: power,
            data: power,
            sts: power,
        }
    }
}

#[non_exhaustive]
pub struct DeviceConfig {
    pub channel: Channel,
    /// Receiver operating parameter set.
    pub rx_ops: RxOps,
    /// Preamble acquisition chunk size.
    pub pac: PreambleAcquisitionChunk,
    pub tx_power: TxPowerConfig,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            channel: Channel::Ch5,
            // DW3000 User Manual section 8.2.12.7 recommends this as default over POR.
            rx_ops: RxOps::ShortPreamble,
            pac: PreambleAcquisitionChunk::Symbols8,
            tx_power: TxPowerConfig::default(),
        }
    }
}

impl<IF: Interface> Dw3000Phy<IF> {
    pub async fn init(
        interface: IF,
        dev_config: DeviceConfig,
    ) -> Result<Self, Error<IF::Error, DeviceError>> {
        let mut interface = InterfaceWrapper(interface);
        reset_power_up(&mut interface).await?;
        check_dev_id(&mut interface).await?;
        let otp = OtpData::load(interface.otp())?;
        interface.set_reset()?;

        Ok(Self {
            interface,
            otp,
            dev_config,
            state: InnerState::Stopped,
        })
    }

    fn shutdown(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        self.interface.set_reset()
    }

    fn configure_ldo(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        if self.otp.ldotune_cal_set {
            // Unprogrammed (zero) value can permanently damage the device
            ral.otp_cfg().write(|w| w.set_ldo_kick(true))?;
        }
        // TODO: change to write a single byte only
        ral.ldo_rload().write_bytes(0x14)?;
        Ok(())
    }

    fn configure_bias(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        if self.otp.bias_tune_byte2 != 0 {
            let mut ral = self.interface.ral();
            ral.otp_cfg().write(|w| w.set_bias_kick(true))?;

            // The kick procedure does not copies all bits, see
            // https://gist.github.com/egnor/455d510e11c22deafdec14b09da5bf54
            // https://forum.qorvo.com/t/missing-and-ambiguous-information-in-dw3000-user-manual/11339/6
            ral.bias_ctrl()
                .modify(|w| w.set_manual_bits(self.otp.bias_tune_byte2 & 0x1f))?;
        }
        Ok(())
    }

    fn configure_xtal_trim(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        const DEFAULT_TRIM_VALUE: u8 = 0x2E;
        let trim_value = if self.otp.xtal_trim != 0 {
            self.otp.xtal_trim
        } else {
            DEFAULT_TRIM_VALUE
        };

        let mut ral = self.interface.ral();
        ral.xtal().write(|w| w.set_xtal_trim(trim_value))?;
        Ok(())
    }

    // TODO: accept lock_code
    async fn start_pll(&mut self, channel: Channel) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        ral.pll_cfg().write_bytes(match channel {
            Channel::Ch5 => 0x1F3C,
            Channel::Ch9 => 0x0F3C,
        })?;

        // Note, user manual prescribes such logic, however API skips it
        let lock_code = self.otp.pll_lock_code;
        if lock_code != 0 {
            ral.pll_cc().modify(|w| w.set_code(lock_code))?;
        }

        ral.pll_cal().write(|w| {
            w.set_pll_cfg_ld(0x8);
            w.set_use_old(lock_code != 0);
        })?;

        // Allow switch to IDLE
        self.interface.clear_events(Events::CPLOCK)?;

        let mut ral = self.interface.ral();
        ral.seq_ctrl().modify(|w| w.set_ainit2idle(true))?;

        self.interface.set_event_mask(Events::CPLOCK)?;
        if !self.interface.wait_for_events(PLL_LOCK_TIMEOUT_US).await? {
            return Err(Error::Device(DeviceError::PllLockTimeout));
        }
        Ok(())
    }

    fn configure_rf(&mut self, config: RfConfig) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        if self.otp.rx_tune_set {
            ral.otp_cfg().write(|w| {
                w.set_dgc_sel(match config.channel {
                    Channel::Ch5 => regs::Channel::Channel5,
                    Channel::Ch9 => regs::Channel::Channel9,
                });
                w.set_dgc_kick(true);
            })?;
        } else {
            ral.dgc_cfg_lut().write(0, &RX_TUNE_DGC_CFG[..8])?;
            let dgc_lut = match config.channel {
                Channel::Ch5 => &RX_TUNE_DGC_LUT_CH5,
                Channel::Ch9 => &RX_TUNE_DGC_LUT_CH9,
            };
            ral.dgc_lut().write(0, dgc_lut)?;
        }

        let mut ral = self.interface.ral();
        ral.otp_cfg().write(|w| {
            w.set_ops_sel(match config.rx_ops {
                RxOps::ShortPreamble => regs::ReceiverParameterSet::Short,
                RxOps::LongPreamble => regs::ReceiverParameterSet::Long,
            });
            w.set_ops_kick(true);
        })?;

        ral.dgc_cfg().write(|w| {
            w.set_rx_tune_en(config.prf == phy::Prf::Mhz64);
            w.set_thr_64(0x32);
        })?;

        ral.dtune0().write(|w| {
            w.set_pac(match config.pac {
                PreambleAcquisitionChunk::Symbols4 => regs::Pac::Symbols4,
                PreambleAcquisitionChunk::Symbols8 => regs::Pac::Symbols8,
                PreambleAcquisitionChunk::Symbols16 => regs::Pac::Symbols16,
                PreambleAcquisitionChunk::Symbols32 => regs::Pac::Symbols32,
            });
            w.set_dt0b4(true);
        })?;

        // User manual prescribes to always set new value, however API updates it for no-data STS only
        ral.dtune3().write_bytes(0xaf5f_35cc)?;

        // User manual does not describe this register, however API changes its value
        if config.channel == Channel::Ch9 {
            ral.rf_rx_ctrl_hi().write_bytes(0x08b5_a833)?;
        }
        ral.rf_tx_ctrl1().write_bytes(0x0e)?;
        ral.rf_tx_ctrl2().write_bytes(match config.channel {
            Channel::Ch5 => 0x1C071134,
            Channel::Ch9 => 0x1C010034,
        })?;
        ral.tx_power().write(|w| {
            w.set_data_pwr(self.dev_config.tx_power.data.0);
            w.set_phr_pwr(self.dev_config.tx_power.phr.0);
            w.set_shr_pwr(self.dev_config.tx_power.shr.0);
            w.set_sts_pwr(self.dev_config.tx_power.sts.0);
        })?;

        Ok(())
    }

    async fn calibrate_rx(&mut self) -> Result<(), Error<IF::Error, DeviceError>> {
        use regs::CalStatus;
        let mut ral = self.interface.ral();

        // Force enable LDO for calibration, see
        // https://gist.github.com/egnor/455d510e11c22deafdec14b09da5bf54#receiver-calibration
        let ldo_ctrl_orig = ral.ldo_ctrl().read()?;
        let ldo_ctrl_calib = {
            let mut w = ldo_ctrl_orig;
            w.set_vddif2_en(true);
            w.set_vddms3_en(true);
            w.set_vddms1_en(true);
            w
        };
        ral.ldo_ctrl().write_value(ldo_ctrl_calib)?;

        ral.rx_cal_sts().clear_bytes(CalStatus::CAL_DONE.bits())?;
        ral.rx_cal().write(|w| {
            w.set_cal_mode(regs::CalibrationMode::Calibration);
            w.set_comp_dly(0x2);
            w.set_cal_en(true);
        })?;

        let mut timeout_us = RX_CALIBRATION_TIMEOUT_US;
        let status = loop {
            let poll_delay_us = core::cmp::min(timeout_us, RX_CALIBRATION_POLL_PERIOD_US);
            self.interface.delay_us(poll_delay_us).await;

            let mut ral = self.interface.ral();
            let status = CalStatus::from_bits_truncate(ral.rx_cal_sts().read_bytes()?);
            if status.contains(CalStatus::CAL_DONE) || timeout_us <= poll_delay_us {
                break status;
            }
            timeout_us -= poll_delay_us;
        };

        let mut ral = self.interface.ral();
        ral.rx_cal().modify(|w| {
            w.set_cal_mode(regs::CalibrationMode::NormalOperation);
            w.set_comp_dly(0x3); // enable readying of RESI/Q
            w.set_cal_en(false);
        })?;

        let resi = ral.rx_cal_resi().read()?.resi();
        let resq = ral.rx_cal_resq().read()?.resq();

        // restore LDO control
        ral.ldo_ctrl().write_value(ldo_ctrl_orig)?;

        if !status.contains(CalStatus::CAL_DONE) {
            return Err(Error::Device(DeviceError::RxCalibrationTimeout));
        }

        const FAILURE_MARKER: u32 = 0x1fff_ffff;
        if resi == FAILURE_MARKER || resq == FAILURE_MARKER {
            return Err(Error::Device(DeviceError::RxCalibrationFailure));
        }

        Ok(())
    }

    fn set_channel(&mut self, config: ChannelConfig) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        ral.chan_ctrl().write(|w| {
            w.set_rf_chan(match config.channel {
                Channel::Ch5 => regs::Channel::Channel5,
                Channel::Ch9 => regs::Channel::Channel9,
            });
            w.set_sfd_type(match config.sfd_type {
                phy::SfdType::Sfd0 => regs::SfdType::Ieee802154,
                phy::SfdType::Sfd2 => regs::SfdType::Ieee802154z,
            });
            w.set_tx_pcode(config.tx_code.as_number());
            w.set_rx_pcode(config.rx_code.as_number());
        })?;
        Ok(())
    }

    fn set_tx_config(
        &mut self,
        config: phy::TxConfig,
        length: u16,
    ) -> Result<(), Error<IF::Error, DeviceError>> {
        const MAX_FRAME_LENGTH: u16 = 1023;
        assert!(length <= MAX_FRAME_LENGTH);

        // TODO: write lower four bytes only
        let mut ral = self.interface.ral();
        // Note, setting the txb_offset offset field is non-trivial.
        // Check errata for correct procedure.
        ral.tx_fctrl_short().write(|w| {
            w.set_txflen(length);
            w.set_txbr(match config.bit_rate {
                phy::BitRate::Kbs850 => regs::BitRate::Kbs850,
                phy::BitRate::Kbs6810 => regs::BitRate::Kbs6810,
            });
            w.set_tr(config.ranging_flag);
            w.set_txpsr(match config.preamble_length {
                phy::PreambleLength::Symbols16 => regs::TxPreambleLength::Symbols16,
                phy::PreambleLength::Symbols32 => regs::TxPreambleLength::Symbols32,
                phy::PreambleLength::Symbols64 => regs::TxPreambleLength::Symbols64,
                phy::PreambleLength::Symbols128 => regs::TxPreambleLength::Symbols128,
                phy::PreambleLength::Symbols256 => regs::TxPreambleLength::Symbols256,
                phy::PreambleLength::Symbols512 => regs::TxPreambleLength::Symbols512,
                phy::PreambleLength::Symbols1024 => regs::TxPreambleLength::Symbols1024,
                phy::PreambleLength::Symbols1536 => regs::TxPreambleLength::Symbols1536,
                phy::PreambleLength::Symbols2048 => regs::TxPreambleLength::Symbols2048,
                phy::PreambleLength::Symbols4096 => regs::TxPreambleLength::Symbols4096,
            });
        })?;

        Ok(())
    }

    // In unlucky case, device can stuck in TX state without neither HPDWARN nor TXFRS events
    // Check user manual 9.4.1 (p.240)
    fn is_tx_missed_deadline_state(&mut self) -> Result<bool, Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        let state = ral.sys_state().read()?;

        const PMSC_TX: u8 = 0xd; // Particular TX state
        const TX_IDLE: u8 = 0x0;
        Ok(state.pmsc_state() == PMSC_TX && state.tx_state() == TX_IDLE)
    }

    fn set_preamble_timeout(
        &mut self,
        pac: PreambleAcquisitionChunk,
        timeout: Option<NonZeroU16>,
    ) -> Result<(), Error<IF::Error, DeviceError>> {
        let preamble_toc = timeout.map_or(0, |count| {
            u16::from(count).div_ceil(pac.as_symbols().into())
        });
        let mut ral = self.interface.ral();
        ral.pre_toc().write_bytes(preamble_toc)?;
        Ok(())
    }

    fn set_rx_frame_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<(), Error<IF::Error, DeviceError>> {
        assert!(timeout <= MAX_RX_FRAME_TIMEOUT);
        const UNIT: Duration = RX_FRAME_TIMEOUT_UNIT;
        let counter = timeout.as_ticks().div_ceil(UNIT.as_ticks());
        assert!(counter < 1u64 << 20);

        let mut ral = self.interface.ral();
        ral.rx_fwto().write(|w| w.set_rx_fwto(counter as u32))?;
        Ok(())
    }

    fn set_sfd_timeout(
        &mut self,
        max_preamble_length: phy::PreambleLength,
        sfd_type: phy::SfdType,
    ) -> Result<(), Error<IF::Error, DeviceError>> {
        // UserManual discourage disabling timeout
        // DW3000 UM, 8.2.7.2
        let max_preamble_length = max_preamble_length.as_symbols();
        let sfd_length: u16 = sfd_type.symbol_length().into();
        let pac_size: u16 = self.dev_config.pac.as_symbols().into();
        let symbols = max_preamble_length.max(pac_size) - pac_size + sfd_length + 1;

        let mut ral = self.interface.ral();
        assert_ne!(symbols, 0);
        ral.rx_sfd_toc().write_bytes(symbols)?;
        Ok(())
    }

    fn set_dx_time(&mut self, time: Instant) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        let sys_ticks_x2 = unwrap!(u32::try_from(
            time.as_ticks() / SYSTEM_TIME_UNIT.as_ticks() * 2
        ));
        ral.dx_time().write_bytes(sys_ticks_x2)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn set_dref_time(&mut self, time: Instant) -> Result<(), Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        let sys_ticks_x2 = unwrap!(u32::try_from(
            time.as_ticks() / SYSTEM_TIME_UNIT.as_ticks() * 2
        ));
        ral.dref_time().write_bytes(sys_ticks_x2)?;
        Ok(())
    }

    fn get_rx_frame_info(
        &mut self,
        sfd_type: phy::SfdType,
        phr_format: phy::PhrFormat,
    ) -> Result<RxFrameInfo, Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        let rx_finfo = ral.rx_finfo().read()?;

        let preamble_length_phr = match rx_finfo.rxpsr() {
            regs::RxPreambleLength::Symbols16 => phy::PreambleLength::Symbols16,
            regs::RxPreambleLength::Symbols64 => phy::PreambleLength::Symbols64,
            regs::RxPreambleLength::Symbols1024 => phy::PreambleLength::Symbols1024,
            regs::RxPreambleLength::Symbols4096 => phy::PreambleLength::Symbols4096,
        };

        let sfd_length = u16::from(sfd_type.symbol_length());
        let acc_count = rx_finfo.rxpacc().max(sfd_length) - sfd_length;
        let preamble_length_acc = match acc_count {
            0..=16 => phy::PreambleLength::Symbols16,
            17..=32 => phy::PreambleLength::Symbols32,
            33..=64 => phy::PreambleLength::Symbols64,
            65..=128 => phy::PreambleLength::Symbols128,
            129..=256 => phy::PreambleLength::Symbols256,
            257..=512 => phy::PreambleLength::Symbols512,
            513..=1024 => phy::PreambleLength::Symbols1024,
            1025..=1536 => phy::PreambleLength::Symbols1536,
            1537..=2048 => phy::PreambleLength::Symbols2048,
            2049.. => phy::PreambleLength::Symbols4096,
        };

        Ok(RxFrameInfo {
            frame_length: rx_finfo.rxflen(),
            bit_rate: match rx_finfo.rxbr() {
                regs::BitRate::Kbs850 => phy::BitRate::Kbs850,
                regs::BitRate::Kbs6810 => phy::BitRate::Kbs6810,
            },
            ranging_flag: rx_finfo.rng(),
            preamble_length_phr: match phr_format {
                phy::PhrFormat::Standard => Some(preamble_length_phr),
                phy::PhrFormat::Long => None,
            },
            preamble_length_acc,
        })
    }

    fn get_sys_timestamp(&mut self) -> Result<Instant, Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();

        // Sys_time is latch during read (and probably some other operations)
        // Make a dummy write transaction to clear the latch
        ral.scratch_mem().write_fast(&[])?;

        let sys_ticks_x2 = ral.sys_time().read_bytes()?;
        Ok(unwrap!(Instant::try_from_ticks(
            (SYSTEM_TIME_UNIT * (sys_ticks_x2 / 2)).as_ticks()
        )))
    }

    fn get_fine_rx_timestamp(&mut self) -> Result<Instant, Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        let rx_time = ral.rx_time().read_bytes()?;
        Ok(unwrap!(Instant::try_from_ticks(rx_time)))
    }

    #[allow(dead_code)]
    fn get_coarse_rx_timestamp(&mut self) -> Result<Instant, Error<IF::Error, DeviceError>> {
        let mut ral = self.interface.ral();
        let sys_ticks_x2 = ral.rx_rawst().read_bytes()?;
        Ok(unwrap!(Instant::try_from_ticks(
            (SYSTEM_TIME_UNIT * (sys_ticks_x2 / 2)).as_ticks()
        )))
    }
}

impl<IF: Interface> phy::Phy for Dw3000Phy<IF> {
    type Instant = Instant;
    type IoError = IF::Error;
    type DevError = DeviceError;

    const MAX_RX_FRAME_TIMEOUT: Duration = MAX_RX_FRAME_TIMEOUT;

    fn state(&self) -> phy::State {
        self.state.into()
    }

    // TODO: go to sleep instead of shutdown
    async fn stop(&mut self) -> Result<(), Error<Self::IoError, Self::DevError>> {
        self.shutdown()?;
        self.state = InnerState::Stopped;
        Ok(())
    }

    async fn start(
        &mut self,
        run_config: phy::RunConfig,
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        // TODO: Check for updates at https://gist.github.com/egnor/455d510e11c22deafdec14b09da5bf54

        let channel_config = ChannelConfig {
            channel: self.dev_config.channel,
            rx_code: run_config.preamble_code,
            tx_code: run_config.preamble_code,
            sfd_type: run_config.sfd_type,
        };

        let base_config = RfConfig {
            channel: self.dev_config.channel,
            prf: run_config.preamble_code.prf(),
            pac: self.dev_config.pac,
            rx_ops: self.dev_config.rx_ops,
        };

        reset_power_up(&mut self.interface).await?;

        let mut ral = self.interface.ral();
        ral.sys_enable_high().write_bytes(0)?;

        self.set_channel(channel_config)?;
        self.configure_ldo()?;
        self.configure_bias()?;
        self.configure_xtal_trim()?;
        self.start_pll(base_config.channel).await?;
        self.configure_rf(base_config)?;
        self.calibrate_rx().await?;

        let mut ral = self.interface.ral();
        ral.sys_cfg().write(|w| {
            w.set_dis_fcs_tx(!run_config.correct_tx_fcs);
            w.set_phr_mode(match run_config.phr_format {
                phy::PhrFormat::Standard => regs::PhrMode::StandardFrame,
                phy::PhrFormat::Long => regs::PhrMode::LongFrame,
            });
            w.set_phr_6m8(run_config.high_phr_bit_rate);
            w.set_cia_ipatov(true);
            w.set_cia_sts(false);
            w.set_rxwtoe(true); // Receive Wait Timeout Enable
            w.set_cp_spc(regs::StsPocketPosition::NoSts);
            w.set_fast_aat(false); // RXFR waits for CIADONE or 
        })?;

        self.interface.clear_all_events()?;

        self.state = InnerState::Running(run_config);

        Ok(())
    }

    async fn get_timestamp(&mut self) -> Result<Instant, Error<Self::IoError, Self::DevError>> {
        if !matches!(self.state, InnerState::Running(_)) {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        }

        self.get_sys_timestamp()
    }

    async fn write_tx_buffer(
        &mut self,
        psdu: &[u8],
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_config) = &self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        if psdu.len() > usize::from(run_config.phr_format.max_psdu_length()) {
            return Err(Error::Operation(OpError::BufferAccessBeyondPhrFormat(
                psdu.len(),
                run_config.phr_format,
            )));
        }

        let mut ral = self.interface.ral();
        ral.tx_buffer().write_fast(psdu)?;
        Ok(())
    }

    async fn read_rx_buffer(
        &mut self,
        psdu: &mut [u8],
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_config) = &self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        if psdu.len() > usize::from(run_config.phr_format.max_psdu_length()) {
            return Err(Error::Operation(OpError::BufferAccessBeyondPhrFormat(
                psdu.len(),
                run_config.phr_format,
            )));
        }

        let mut ral = self.interface.ral();
        ral.rx_buffer0().read_fast(psdu)?;
        Ok(())
    }

    async fn transmit(
        &mut self,
        tx_config: phy::TxConfig,
        length: u16,
        start_at: Instant,
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_config) = self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        if length > run_config.phr_format.max_psdu_length() {
            return Err(Error::Operation(OpError::TxLengthAbovePhrFormat(
                length,
                run_config.phr_format,
            )));
        }

        if run_config.correct_tx_fcs && length < phy::FCS_LENGTH {
            return Err(Error::Operation(OpError::TxLengthLessThanFcs(length)));
        }

        let shr_duration = phy::shr_duration(
            run_config.preamble_code.prf(),
            run_config.sfd_type,
            tx_config.preamble_length,
        );
        let rmarker_at = start_at + shr_duration;

        self.set_tx_config(tx_config, length)?;
        self.set_dx_time(rmarker_at)?;

        self.interface.send_command(FastCommand::Dtx)?;
        let start_instant = self.get_sys_timestamp()?;
        let overtime = start_instant - start_at;

        let imm_events = self.interface.get_events()?;
        if imm_events.contains(Events::HPDWARN) {
            self.interface.send_command(FastCommand::Txrxoff)?;
            self.interface.clear_all_events()?;
            return Err(OpError::StartInstantPassed(overtime).into());
        }

        let bug_state = self.is_tx_missed_deadline_state()?;
        if bug_state {
            self.interface.send_command(FastCommand::Txrxoff)?;
            self.interface.clear_all_events()?;
            return Err(OpError::StartInstantPassed(overtime).into());
        }

        let phr_bit_rate = if run_config.high_phr_bit_rate {
            tx_config.bit_rate
        } else {
            phy::BitRate::Kbs850
        };

        const _START_DELAY_MAX: Duration = Instant::PERIOD;
        let start_delay = start_at - start_instant;

        const _FRAME_DURATION_MAX: Duration = Instant::PERIOD; // significant exaggeration
        let frame_duration = shr_duration
            + phy::phr_duration(phr_bit_rate)
            + phy::psdu_duration(tx_config.bit_rate, length);

        const _EVENT_TIMEOUT_US_MAX: u64 = _START_DELAY_MAX
            .add(_FRAME_DURATION_MAX)
            .div_ceil(HOST_MICROSECOND_MIN);
        let event_timeout_us = (start_delay + frame_duration).div_ceil(HOST_MICROSECOND_MIN);

        const _ASSERT: u64 = u32::MAX as u64 - _EVENT_TIMEOUT_US_MAX;
        let event_timeout_us = unwrap!(u32::try_from(event_timeout_us));

        self.interface.set_event_mask(Events::TXFRS)?;
        if !self.interface.wait_for_events(event_timeout_us).await? {
            self.interface.send_command(FastCommand::Txrxoff)?;
            self.interface.clear_all_events()?;
            return Err(Error::Device(DeviceError::TxStateTimeout {
                timeout_us: event_timeout_us,
            }));
        }

        self.interface.clear_all_events()?;

        Ok(())
    }

    async fn receive(
        &mut self,
        rx_config: phy::RxConfig,
        start_at: Instant,
        rx_timeout: Duration,
    ) -> Result<Option<phy::RxReport<Self::Instant>>, Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_config) = self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        if rx_timeout > MAX_RX_FRAME_TIMEOUT {
            return Err(Error::Operation(OpError::ExcessiveRxTimeout(rx_timeout)));
        }

        self.set_sfd_timeout(rx_config.max_preamble_length, run_config.sfd_type)?;
        self.set_preamble_timeout(self.dev_config.pac, rx_config.max_preamble_hunt)?;
        self.set_dx_time(start_at)?;
        self.set_rx_frame_timeout(rx_timeout)?;

        self.interface.send_command(FastCommand::Drx)?;
        let start_instant = self.get_sys_timestamp()?;
        let overtime = start_instant - start_at;

        let imm_events = self.interface.get_events()?;
        if imm_events.contains(Events::HPDWARN) {
            self.interface.send_command(FastCommand::Txrxoff)?;
            self.interface.clear_all_events()?;
            return Err(OpError::StartInstantPassed(overtime).into());
        }

        self.interface.set_event_mask(RX_TERMINATION_EVENTS)?;

        const _START_DELAY_MAX: Duration = Instant::PERIOD;
        let start_delay = start_at - start_instant;

        const _EVENT_TIMEOUT_US_MAX: u64 = _START_DELAY_MAX
            .add(MAX_RX_FRAME_TIMEOUT)
            .div_ceil(HOST_MICROSECOND_MIN);
        let event_timeout_us = (start_delay + rx_timeout).div_ceil(HOST_MICROSECOND_MIN);

        const _ASSERT: u64 = u32::MAX as u64 - _EVENT_TIMEOUT_US_MAX;
        let event_timeout_us = unwrap!(u32::try_from(event_timeout_us));

        if !self.interface.wait_for_events(event_timeout_us).await? {
            self.interface.send_command(FastCommand::Txrxoff)?;
            self.interface.clear_all_events()?;
            return Err(Error::Device(DeviceError::RxStateTimeout {
                timeout_us: event_timeout_us,
            }));
        }
        let events = self.interface.get_events()?;
        self.interface.clear_all_events()?;

        // TODO: Add RX error signalling
        if !events.contains(Events::RXFR | Events::CIADONE) {
            return Ok(None);
        }

        let frame_info = self.get_rx_frame_info(run_config.sfd_type, run_config.phr_format)?;
        let timestamp = self.get_fine_rx_timestamp()?;

        Ok(Some(phy::RxReport {
            preamble_length_phr: frame_info.preamble_length_phr,
            preamble_length_acc: frame_info.preamble_length_acc,
            bit_rate: frame_info.bit_rate,
            ranging_flag: frame_info.ranging_flag,
            length: frame_info.frame_length,
            fcs_good: events.contains(Events::RXFCG),
            timestamp,
        }))
    }
}
