use crate::interface::Interface;
use crate::otp;
use crate::phy::{self, time::Duration};
use crate::ral::{self, RegisterAccess, regs};
use core::num::NonZeroU16;

pub use regs::{Channel, EventsLow as Events, FrameFormat, PacSize, SfdType};

// 124.8 MHZ system clock period
const SYSTEM_TIME_UNIT: Duration = Duration::CHIP.mul_u32(4);
// 31-bit system clock counter
const SYSTEM_TIME_PERIOD: Duration = SYSTEM_TIME_UNIT.mul_u32(1u32 << 31);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Timebase;

impl phy::time::CyclicTimebase for Timebase {
    const PERIOD: Duration = SYSTEM_TIME_PERIOD;
    const SCHEDULE_QUANT: Duration = SYSTEM_TIME_UNIT;
}

pub type Instant = phy::time::Instant<Timebase>;

const RX_FRAME_TIMEOUT_UNIT: Duration = Duration::CHIP.mul_u32(512);
pub const MAX_FRAME_TIMEOUT: Duration = RX_FRAME_TIMEOUT_UNIT.mul_u32((1u32 << 20) - 1);

#[rustfmt::skip]
static RX_TUNE_DGC_CFG: regs::DgcCfgLutData = [
    0x40, 0x02, 0x00, 0x10,
    0x89, 0xa4, 0x6d, 0x1b,
    0x23, 0xc9, 0xb6, 0x2d,
    0xb5, 0x6d, 0x20, 0x12,
    0xda, 0xb6, 0x91, 0x24,
    0x24, 0x49, 0xb6, 0x2d,
    0x6d, 0xdb, 0x16, 0x00,
];

#[rustfmt::skip]
static RX_TUNE_DGC_LUT_CH5: regs::DgcLutData = [
    0xfd, 0xc0, 0x01, 0x00,
    0x3e, 0xc4, 0x01, 0x00,
    0xbe, 0xc6, 0x01, 0x00,
    0x7e, 0xc7, 0x01, 0x00,
    0x36, 0xcf, 0x01, 0x00,
    0xb5, 0xcf, 0x01, 0x00,
    0xf5, 0xcf, 0x01, 0x00,
];

#[rustfmt::skip]
static RX_TUNE_DGC_LUT_CH9: regs::DgcLutData = [
    0xfe, 0xa8, 0x02, 0x00,
    0x36, 0xac, 0x02, 0x00,
    0xfe, 0xa5, 0x02, 0x00,
    0x3e, 0xaf, 0x02, 0x00,
    0x7d, 0xaf, 0x02, 0x00,
    0xb5, 0xaf, 0x02, 0x00,
    0xb5, 0xaf, 0x02, 0x00,
];

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FastCommand {
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

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MeanPrf {
    /// 31-symbols preambles, CHIP_FREQ / 16
    Mhz16,
    /// 127-symbols preambles, CHIP_FREQ / 4
    Mhz62,
}

impl MeanPrf {
    pub const fn code_range(self) -> [u8; 2] {
        match self {
            MeanPrf::Mhz16 => [1, 8],
            MeanPrf::Mhz62 => [9, 24],
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BitRate {
    /// 0.85 MBit/s for PHR and payload
    Kbs850,
    /// 0.85 MBit/s for PHR, 6.81 MBit/s for payload, a.k.a. DRBM_LP
    Kbs6810,
    /// 6.81 MBit/s for PHR and payload, a.k.a. DRBM_HP
    Kbs6810Only,
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

trait RalInterface: Interface {
    fn ral(&mut self) -> RegisterAccess<'_, Self>
    where
        Self: Sized,
    {
        RegisterAccess::new(self)
    }
}

impl<IF: Interface> RalInterface for IF {}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    pub channel: Channel,
    pub prf: MeanPrf,
    pub rx_code: u8,
    pub tx_code: u8,
    pub rx_psr: phy::Psr,
    pub pac_size: PacSize,
    pub sfd_type: SfdType,
    pub cia_enable: bool,
    pub bit_rate: BitRate,
    pub frame_format: FrameFormat,
    pub correct_tx_fcs: bool,
    pub tx_power: TxPowerConfig,
}

struct OtpData {
    ldotune_cal_set: bool,
    xtal_trim: u8,
    bias_tune_byte2: u8,
    rx_tune_set: bool,
    pll_lock_code: u32,
}

impl OtpData {
    fn load<IF: Interface>(interface: &mut IF) -> Result<Self, Error<IF>> {
        let mut otp = otp::OtpReader::new(interface);
        Ok(Self {
            ldotune_cal_set: otp.ldotune_cal()? != 0,
            xtal_trim: otp.xtal_trim()?,
            bias_tune_byte2: otp.bias_tune()?.to_le_bytes()[2],
            rx_tune_set: otp.rx_tune_dgc_cfg0()?.to_le_bytes() == RX_TUNE_DGC_CFG[0..4],
            pll_lock_code: otp.pll_lock_code()?,
        })
    }
}

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
    UnmaskedEvent { mask: Events, events: Events },
    NoAccumulatedPreambles,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<IF: Interface> {
    Interface(IF::Error),
    Device(DeviceError),
}

impl<IF: Interface> From<ral::Error<IF>> for Error<IF> {
    fn from(value: ral::Error<IF>) -> Self {
        match value {
            ral::Error::Interface(err) => Self::Interface(err),
        }
    }
}

pub struct Device<IF> {
    interface: IF,
    otp: OtpData,
}

impl<IF: Interface> Device<IF> {
    pub async fn init(interface: IF) -> Result<Self, Error<IF>> {
        let mut interface = interface;
        reset_power_up(&mut interface).await?;
        check_dev_id(&mut interface).await?;
        let otp = OtpData::load(&mut interface)?;
        interface.set_reset().map_err(Error::Interface)?;

        Ok(Self { interface, otp })
    }

    pub fn shutdown(&mut self) -> Result<(), Error<IF>> {
        self.interface.set_reset().map_err(Error::Interface)
    }

    pub async fn configure(&mut self, config: Config) -> Result<(), Error<IF>> {
        reset_power_up(&mut self.interface).await?;

        self.clear_high_event_mask()?;
        self.set_channel(&config)?;
        self.configure_ldo()?;
        self.configure_bias()?;
        self.configure_xtal_trim()?;
        self.start_pll(config.channel).await?;
        self.configure_rf(&config)?;
        self.calibrate_rx().await?;
        self.set_sfd_timeout(config.pac_size, config.sfd_type, config.rx_psr)?;
        self.set_diagnostic_enable(true)?; // cia power access

        let mut ral = self.interface.ral();
        ral.sys_cfg().write(|w| {
            w.set_dis_fcs_tx(!config.correct_tx_fcs);
            w.set_phr_mode(config.frame_format);
            w.set_phr_6m8(config.bit_rate == BitRate::Kbs6810Only);
            w.set_cia_ipatov(config.cia_enable);
            w.set_cia_sts(false);
            w.set_rxwtoe(true); // Receive Wait Timeout Enable
            w.set_cp_spc(regs::StsPocketPosition::NoSts);
            w.set_fast_aat(false); // RXFR waits for CIADONE or CIAERR
        })?;

        Ok(())
    }

    fn clear_high_event_mask(&mut self) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        ral.sys_enable_high().write_bytes(0)?;
        Ok(())
    }

    fn set_channel(&mut self, config: &Config) -> Result<(), Error<IF>> {
        let [min_code, max_code] = config.prf.code_range();
        assert!(
            (min_code..=max_code).contains(&config.rx_code),
            "RX preamble code {} is outside the {:?} range ({}..={})",
            config.rx_code,
            config.prf,
            min_code,
            max_code,
        );
        assert!(
            (min_code..=max_code).contains(&config.tx_code),
            "TX preamble code {} is outside the {:?} range ({}..={})",
            config.tx_code,
            config.prf,
            min_code,
            max_code,
        );

        let mut ral = self.interface.ral();
        ral.chan_ctrl().write(|w| {
            w.set_rf_chan(config.channel);
            w.set_sfd_type(config.sfd_type);
            w.set_tx_pcode(config.tx_code);
            w.set_rx_pcode(config.rx_code);
        })?;
        Ok(())
    }

    fn configure_ldo(&mut self) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        if self.otp.ldotune_cal_set {
            // Unprogrammed (zero) value can permanently damage the device
            ral.otp_cfg().write(|w| w.set_ldo_kick(true))?;
        }
        // TODO: change to write a single byte only
        ral.ldo_rload().write_bytes(0x14)?;
        Ok(())
    }

    fn configure_bias(&mut self) -> Result<(), Error<IF>> {
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

    fn configure_xtal_trim(&mut self) -> Result<(), Error<IF>> {
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
    async fn start_pll(&mut self, channel: Channel) -> Result<(), Error<IF>> {
        const PLL_LOCK_TIMEOUT_US: u32 = 150_000;

        let mut ral = self.interface.ral();
        ral.pll_cfg().write_bytes(match channel {
            Channel::Ch5 => 0x1F3C,
            Channel::Ch9 => 0x0F3C,
        })?;

        // Note, user manual prescribes such logic, however API skips it
        if self.otp.pll_lock_code != 0 {
            ral.pll_cc()
                .modify(|w| w.set_code(self.otp.pll_lock_code))?;
        }

        ral.pll_cal().write(|w| {
            w.set_pll_cfg_ld(0x8);
            w.set_use_old(self.otp.pll_lock_code != 0);
        })?;

        // Allow switch to IDLE
        self.clear_events(Events::CPLOCK)?;

        let mut ral = self.interface.ral();
        ral.seq_ctrl().modify(|w| w.set_ainit2idle(true))?;

        self.set_event_mask(Events::CPLOCK)?;
        if !self.wait_for_events(PLL_LOCK_TIMEOUT_US).await? {
            return Err(Error::Device(DeviceError::PllLockTimeout));
        }
        Ok(())
    }

    fn configure_rf(&mut self, config: &Config) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        if self.otp.rx_tune_set {
            ral.otp_cfg().write(|w| {
                w.set_dgc_sel(config.channel);
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
            w.set_ops_sel(
                // Check DW3000 User Manual section 8.2.12.7 for details
                if config.rx_psr >= phy::Psr::Symbols256 {
                    regs::ReceiverParameterSet::Long
                } else {
                    regs::ReceiverParameterSet::Short
                },
            );
            w.set_ops_kick(true);
        })?;

        ral.dgc_cfg().write(|w| {
            w.set_rx_tune_en(recommended_rx_tune_en(config.prf));
            w.set_thr_64(0x32);
        })?;

        ral.dtune0().write(|w| {
            w.set_pac(config.pac_size);
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
            w.set_data_pwr(config.tx_power.data.0);
            w.set_phr_pwr(config.tx_power.phr.0);
            w.set_shr_pwr(config.tx_power.shr.0);
            w.set_sts_pwr(config.tx_power.sts.0);
        })?;

        Ok(())
    }

    async fn calibrate_rx(&mut self) -> Result<(), Error<IF>> {
        const RX_CALIBRATION_POLL_PERIOD_US: u32 = 20_000;
        const RX_CALIBRATION_TIMEOUT_US: u32 = 60_000;
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

    fn set_sfd_timeout(
        &mut self,
        pac_size: PacSize,
        sfd_type: SfdType,
        max_psr: phy::Psr,
    ) -> Result<(), Error<IF>> {
        // UserManual discourage disabling timeout
        // DW3000 UM, 8.2.7.2
        let max_psr = max_psr.as_symbols();
        let pac_size = pac_size.as_symbols();
        let sfd_length = sfd_type.as_symbols();
        let symbols = max_psr.max(pac_size) - pac_size + sfd_length + 1;

        let mut ral = self.interface.ral();
        assert_ne!(symbols, 0);
        ral.rx_sfd_toc().write_bytes(symbols)?;
        Ok(())
    }

    fn set_diagnostic_enable(&mut self, value: bool) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        ral.cia_conf_byte2().write(|w| w.set_mindiag(!value))?;
        Ok(())
    }

    pub fn set_tx_config(
        &mut self,
        psr: phy::Psr,
        bit_rate: BitRate,
        ranging: bool,
        length: u16,
    ) -> Result<(), Error<IF>> {
        const MAX_FRAME_LENGTH: u16 = 1023;
        assert!(length <= MAX_FRAME_LENGTH);

        // TODO: write lower four bytes only
        let mut ral = self.interface.ral();
        // Note, setting the txb_offset offset field is non-trivial.
        // Check errata for correct procedure.
        ral.tx_fctrl_short().write(|w| {
            w.set_txflen(length);
            w.set_txbr(match bit_rate {
                BitRate::Kbs850 => regs::BitRate::Kbs850,
                BitRate::Kbs6810 | BitRate::Kbs6810Only => regs::BitRate::Kbs6810,
            });
            w.set_tr(ranging);
            w.set_txpsr(match psr {
                phy::Psr::Symbols16 => regs::TxPsr::Symbols16,
                phy::Psr::Symbols32 => regs::TxPsr::Symbols32,
                phy::Psr::Symbols64 => regs::TxPsr::Symbols64,
                phy::Psr::Symbols128 => regs::TxPsr::Symbols128,
                phy::Psr::Symbols256 => regs::TxPsr::Symbols256,
                phy::Psr::Symbols512 => regs::TxPsr::Symbols512,
                phy::Psr::Symbols1024 => regs::TxPsr::Symbols1024,
                phy::Psr::Symbols1536 => regs::TxPsr::Symbols1536,
                phy::Psr::Symbols2048 => regs::TxPsr::Symbols2048,
                phy::Psr::Symbols4096 => regs::TxPsr::Symbols4096,
            });
        })?;

        Ok(())
    }

    pub fn set_preamble_timeout(
        &mut self,
        pac_size: PacSize,
        timeout: Option<NonZeroU16>,
    ) -> Result<(), Error<IF>> {
        let preamble_toc =
            timeout.map_or(0, |count| u16::from(count).div_ceil(pac_size.as_symbols()));
        let mut ral = self.interface.ral();
        ral.pre_toc().write_bytes(preamble_toc)?;
        Ok(())
    }

    pub fn set_rx_frame_timeout(&mut self, timeout: Duration) -> Result<(), Error<IF>> {
        assert!(timeout <= MAX_FRAME_TIMEOUT);
        const UNIT: Duration = RX_FRAME_TIMEOUT_UNIT;
        let counter = timeout.as_ticks().div_ceil(UNIT.as_ticks());
        assert!(counter < 1u64 << 20);

        let mut ral = self.interface.ral();
        ral.rx_fwto().write(|w| w.set_rx_fwto(counter as u32))?;
        Ok(())
    }

    pub fn get_sys_timestamp(&mut self) -> Result<Instant, Error<IF>> {
        let mut ral = self.interface.ral();

        // Sys_time is latch during read (and probably some other operations)
        // Make a dummy write transaction to clear the latch
        ral.scratch_mem().write_fast(&[])?;

        let sys_ticks_x2 = ral.sys_time().read_bytes()?;
        Ok(unwrap!(Instant::try_from_ticks(
            (SYSTEM_TIME_UNIT * (sys_ticks_x2 / 2)).as_ticks()
        )))
    }

    pub fn set_dx_time(&mut self, time: Instant) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        let sys_ticks_x2 = unwrap!(u32::try_from(
            time.as_ticks() / SYSTEM_TIME_UNIT.as_ticks() * 2
        ));
        ral.dx_time().write_bytes(sys_ticks_x2)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_dref_time(&mut self, time: Instant) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        let sys_ticks_x2 = unwrap!(u32::try_from(
            time.as_ticks() / SYSTEM_TIME_UNIT.as_ticks() * 2
        ));
        ral.dref_time().write_bytes(sys_ticks_x2)?;
        Ok(())
    }

    pub fn send_command(&mut self, command: FastCommand) -> Result<(), Error<IF>> {
        self.interface
            .send_command(command as u8)
            .map_err(Error::Interface)
    }

    pub fn set_event_mask(&mut self, mask: Events) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        ral.sys_enable_low().write_bytes(mask.bits())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn has_events(&mut self) -> Result<bool, Error<IF>> {
        self.interface.is_irq().map_err(Error::Interface)
    }

    pub async fn wait_for_events(&mut self, timeout_us: u32) -> Result<bool, Error<IF>> {
        self.interface
            .wait_for_irq(timeout_us)
            .await
            .map_err(Error::Interface)
    }

    pub fn get_events(&mut self) -> Result<Events, Error<IF>> {
        let mut ral = self.interface.ral();
        let value = ral.sys_status_low().read_bytes()?;
        Ok(Events::from_bits_truncate(value))
    }

    pub fn clear_events(&mut self, events: Events) -> Result<(), Error<IF>> {
        let mut ral = self.interface.ral();
        ral.sys_status_low().clear_bytes(events.bits())?;
        Ok(())
    }

    pub fn clear_all_events(&mut self) -> Result<(), Error<IF>> {
        self.send_command(FastCommand::ClrIrqs)?;
        Ok(())
    }

    // In unlucky case, device can stuck in TX state without neither HPDWARN nor TXFRS events
    // Check user manual 9.4.1 (p.240)
    pub fn is_tx_missed_deadline_state(&mut self) -> Result<bool, Error<IF>> {
        let mut ral = self.interface.ral();
        let state = ral.sys_state().read()?;

        const PMSC_TX: u8 = 0xd; // Particular TX state
        const TX_IDLE: u8 = 0x0;
        Ok(state.pmsc_state() == PMSC_TX && state.tx_state() == TX_IDLE)
    }

    pub fn get_rx_frame_length(&mut self) -> Result<u16, Error<IF>> {
        let mut ral = self.interface.ral();
        let rx_info = ral.rx_finfo_short().read()?;

        Ok(rx_info.rxflen())
    }

    pub fn get_coarse_rx_timestamp(&mut self) -> Result<Instant, Error<IF>> {
        let mut ral = self.interface.ral();
        let sys_ticks_x2 = ral.rx_rawst().read_bytes()?;
        Ok(unwrap!(Instant::try_from_ticks(
            (SYSTEM_TIME_UNIT * (sys_ticks_x2 / 2)).as_ticks()
        )))
    }

    pub fn get_fine_rx_timestamp(&mut self) -> Result<Instant, Error<IF>> {
        let mut ral = self.interface.ral();
        let rx_time = ral.rx_time().read_bytes()?;
        Ok(unwrap!(Instant::try_from_ticks(rx_time)))
    }

    pub fn get_first_path_cia_power(&mut self, prf: MeanPrf) -> Result<f32, Error<IF>> {
        let mut ral = self.interface.ral();
        let dgc_shift = ral.dgc_dbg().read()?.dgc_decision();
        let fp1m = ral.ip_diag2().read()?.ip_fp1m();
        let fp2m = ral.ip_diag3().read()?.ip_fp2m();
        let fp3m = ral.ip_diag4().read()?.ip_fp3m();
        let count = ral.ip_diag12().read()?.ip_nacc() << 2; // fp has 2 fractional bits

        let dgc_shift = if recommended_rx_tune_en(prf) {
            dgc_shift
        } else {
            0
        };
        let count =
            NonZeroU16::new(count).ok_or(Error::Device(DeviceError::NoAccumulatedPreambles))?;

        let fp = [fp1m, fp2m, fp3m].map(|x| x << u32::from(dgc_shift));
        let power = first_path_cia_power(prf, fp, count);
        Ok(power)
    }

    pub fn get_full_cia_power(&mut self, prf: MeanPrf) -> Result<f32, Error<IF>> {
        let mut ral = self.interface.ral();
        let dgc_shift = ral.dgc_dbg().read()?.dgc_decision();
        let ir_area = ral.ip_diag1().read()?.ip_crea();
        let count = ral.ip_diag12().read()?.ip_nacc();

        let dgc_shift = if recommended_rx_tune_en(prf) {
            dgc_shift
        } else {
            0
        };
        let count =
            NonZeroU16::new(count).ok_or(Error::Device(DeviceError::NoAccumulatedPreambles))?;

        let shifted_ir_area = ir_area << u32::from(dgc_shift * 2);
        let power = full_cia_power(prf, shifted_ir_area, count);
        Ok(power)
    }

    pub fn read_rx_buffer(&mut self, psdu: &mut [u8]) -> Result<(), Error<IF>> {
        const RX_BUFFER_MAX_SIZE: usize = 1024;
        assert!(psdu.len() <= RX_BUFFER_MAX_SIZE);

        let mut ral = self.interface.ral();
        ral.rx_buffer0().read_fast(psdu)?;
        Ok(())
    }

    pub fn write_tx_buffer(&mut self, psdu: &[u8]) -> Result<(), Error<IF>> {
        const TX_BUFFER_MAX_SIZE: usize = 1024;
        assert!(psdu.len() <= TX_BUFFER_MAX_SIZE);

        let mut ral = self.interface.ral();
        ral.tx_buffer().write_fast(psdu)?;
        Ok(())
    }
}

async fn reset_power_up<IF: Interface>(interface: &mut IF) -> Result<(), Error<IF>> {
    const RESET_DELAY_US: u32 = 1_000;

    interface.set_reset().map_err(Error::Interface)?;
    interface.delay_us(RESET_DELAY_US).await;
    interface.clear_reset().map_err(Error::Interface)?;

    // initialization used to last 1200us
    const XTAL_WAKE_UP_TIMEOUT_US: u32 = 2_000;

    // SPIRDY event is enabled by default
    if !interface
        .wait_for_irq(XTAL_WAKE_UP_TIMEOUT_US)
        .await
        .map_err(Error::Interface)?
    {
        return Err(Error::Device(DeviceError::ResetTimeout));
    }

    let mut ral = interface.ral();
    let events = Events::from_bits_truncate(ral.sys_status_low().read_bytes()?);
    if !events.contains(Events::SPIRDY) {
        return Err(Error::Device(DeviceError::SpiNotReady));
    }
    Ok(())
}

// TODO: tune performance for Symbols32
pub fn recommended_pac_size(psr: phy::Psr) -> PacSize {
    if psr < phy::Psr::Symbols64 {
        PacSize::Symbols4
    } else if psr < phy::Psr::Symbols128 {
        PacSize::Symbols8
    } else {
        PacSize::Symbols16
    }
}

async fn check_dev_id<IF: Interface>(interface: &mut IF) -> Result<(), Error<IF>> {
    /// QORVO Register Identification Tag
    const DEV_RIDTAG: u16 = 0xdeca;
    /// DW3000 device
    const DEV_MODEL: u8 = 0x03;
    /// Non-PDoA device
    const DEV_VERSION: u8 = 0x0;
    const DEV_REVISION: u8 = 0x2;

    let mut ral = interface.ral();
    let id = ral.dev_id().read()?;
    if id.ridtag() != DEV_RIDTAG
        || id.model() != DEV_MODEL
        || id.ver() != DEV_VERSION
        || id.rev() != DEV_REVISION
    {
        return Err(Error::Device(DeviceError::WrongDevice));
    }
    Ok(())
}

// DGC_CFG bit derivation according to UG 8.2.4.1
const fn recommended_rx_tune_en(prf: MeanPrf) -> bool {
    match prf {
        MeanPrf::Mhz16 => false,
        MeanPrf::Mhz62 => true,
    }
}

fn lsb_cia_power(prf: MeanPrf) -> f32 {
    match prf {
        MeanPrf::Mhz16 => 4.168694e-15,  // -113.8 dBm in W
        MeanPrf::Mhz62 => 6.7608296e-16, // -121.7 dBm in W
    }
}

fn first_path_cia_power(prf: MeanPrf, pf: [u32; 3], n: NonZeroU16) -> f32 {
    let pf_sq: f32 = pf
        .into_iter()
        .map(|x| {
            let x_fp = x as f32;
            x_fp * x_fp
        })
        .sum();
    let n_fp = n.get() as f32;
    lsb_cia_power(prf) * pf_sq / (n_fp * n_fp)
}

fn full_cia_power(prf: MeanPrf, c: u32, n: NonZeroU16) -> f32 {
    const SCALE: f32 = (1u32 << 21) as f32;
    let c_fp = c as f32;
    let n_fp = n.get() as f32;
    lsb_cia_power(prf) * SCALE * c_fp / (n_fp * n_fp)
}

impl PacSize {
    const fn as_symbols(self) -> u16 {
        match self {
            PacSize::Symbols4 => 4,
            PacSize::Symbols8 => 8,
            PacSize::Symbols16 => 16,
            PacSize::Symbols32 => 32,
        }
    }
}

impl SfdType {
    pub const fn as_symbols(self) -> u16 {
        match self {
            Self::IeeeSfd0 | Self::Decawave8 | Self::IeeeSfd2 => 8,
            Self::Decawave16 => 16,
        }
    }
}
