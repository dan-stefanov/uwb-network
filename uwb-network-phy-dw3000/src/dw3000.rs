use crate::device::{
    self, DeviceError, FastCommand, Instant, PreambleAcquisitionChunk, Timebase, TxPowerConfig,
};
use crate::interface::Interface;
use crate::phy::{self, Error, OpError, time::CyclicTimebase, time::Duration};
use crate::ral::regs::EventsLow as Events;
use core::num::NonZeroU16;

fn check_start_instant_alignment(start_at: Instant) -> Result<(), OpError> {
    let aligned_start = start_at.schedule_align_down();
    let offset = start_at - aligned_start;
    if offset == Duration::ZERO {
        Ok(())
    } else {
        Err(OpError::StartInstantNotAligned(offset))
    }
}

const CAPABILITIES: phy::Capabilities = phy::Capabilities::CH_5
    .union(phy::Capabilities::CH_9)
    .union(phy::Capabilities::PRF_16)
    .union(phy::Capabilities::PRF_62)
    .union(phy::Capabilities::PSR_32)
    .union(phy::Capabilities::PSR_64)
    .union(phy::Capabilities::PSR_128)
    .union(phy::Capabilities::PSR_256)
    .union(phy::Capabilities::PSR_512)
    .union(phy::Capabilities::PSR_1024)
    .union(phy::Capabilities::PSR_1536)
    .union(phy::Capabilities::PSR_2048)
    .union(phy::Capabilities::PSR_4096)
    .union(phy::Capabilities::SFD_0)
    .union(phy::Capabilities::SFD_2)
    .union(phy::Capabilities::BIT_RATE_850)
    .union(phy::Capabilities::BIT_RATE_6810)
    .union(phy::Capabilities::BIT_RATE_6810_ONLY)
    .union(phy::Capabilities::LONG_FRAME_FORMAT)
    .union(phy::Capabilities::CORRECT_TX_FCS);

// minimum microsecond duration in host system relative to DW3000 clock
const HOST_MICROSECOND_MIN: Duration = {
    const CLOCK_TOL: f32 = 0.05; // STM32 HSI16 are typically below 2%
    let ticks = (Duration::SECOND.as_ticks() as f32 / 1.0e6 / (1.0 + CLOCK_TOL)) as u64;
    core::assert!(ticks > 0);
    Duration::from_ticks(ticks)
};

fn max_psdu_length(long_frame_format: bool) -> u16 {
    if long_frame_format {
        phy::MAX_LONG_PSDU_LENGTH
    } else {
        phy::MAX_PSDU_LENGTH
    }
}

impl<IF: Interface> From<device::LowLevelError<IF>> for Error<IF::Error, DeviceError> {
    fn from(value: device::LowLevelError<IF>) -> Self {
        match value {
            device::LowLevelError::Interface(err) => Self::Interface(err),
            device::LowLevelError::Device(err) => Self::Device(err),
        }
    }
}

const RX_TERMINATION_EVENTS: Events = Events::RXPHE
    .union(Events::RXFR)
    .union(Events::RXFSL)
    .union(Events::RXFTO)
    .union(Events::RXPTO)
    .union(Events::RXSTO)
    .union(Events::ARFE);

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct RunData {
    config: phy::RunConfig,
    pac: PreambleAcquisitionChunk,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum InnerState {
    Stopped,
    Running(RunData),
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
    device: device::Device<IF>,
    dev_config: DeviceConfig,
    state: InnerState,
}

// TODO: add XTAL trim option
#[non_exhaustive]
pub struct DeviceConfig {
    pub tx_power: TxPowerConfig,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            tx_power: TxPowerConfig::default(),
        }
    }
}

impl<IF: Interface> Dw3000Phy<IF> {
    pub async fn init(
        interface: IF,
        dev_config: DeviceConfig,
    ) -> Result<Self, Error<IF::Error, DeviceError>> {
        Ok(Self {
            device: device::Device::init(interface).await?,
            dev_config,
            state: InnerState::Stopped,
        })
    }
}

impl<IF: Interface> phy::Phy for Dw3000Phy<IF> {
    type Timebase = Timebase;
    type IoError = IF::Error;
    type DevError = DeviceError;

    fn state(&self) -> phy::State {
        self.state.into()
    }

    fn capabilities(&self) -> phy::Capabilities {
        CAPABILITIES
    }

    fn max_rx_timeout(&self) -> Duration {
        device::MAX_RX_TIMEOUT
    }

    // TODO: go to sleep instead of shutdown
    async fn stop(&mut self) -> Result<(), Error<Self::IoError, Self::DevError>> {
        self.device.shutdown()?;
        self.state = InnerState::Stopped;
        Ok(())
    }

    async fn start(
        &mut self,
        run_config: phy::RunConfig,
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        // TODO: Check for updates at https://gist.github.com/egnor/455d510e11c22deafdec14b09da5bf54

        run_config
            .check_capabilities(self.capabilities())
            .map_err(OpError::UnsupportedConfig)?;

        let pac = device::recommended_pac_length(run_config.psr);
        self.device
            .configure(run_config, &self.dev_config, pac)
            .await?;

        self.device.clear_all_events()?;

        self.state = InnerState::Running(RunData {
            config: run_config,
            pac,
        });

        Ok(())
    }

    async fn get_timestamp(&mut self) -> Result<Instant, Error<Self::IoError, Self::DevError>> {
        if !matches!(self.state, InnerState::Running(_)) {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        }

        Ok(self.device.get_sys_timestamp()?)
    }

    async fn write_tx_buffer(
        &mut self,
        psdu: &[u8],
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_data) = &self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        let max_psdu_length = max_psdu_length(run_data.config.long_frame_format);
        if psdu.len() > usize::from(max_psdu_length) {
            return Err(Error::Operation(OpError::BufferAccessBeyondFrameFormat(
                psdu.len(),
                max_psdu_length,
            )));
        }

        self.device.write_tx_buffer(psdu)?;
        Ok(())
    }

    async fn read_rx_buffer(
        &mut self,
        psdu: &mut [u8],
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_data) = &self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        let max_psdu_length = max_psdu_length(run_data.config.long_frame_format);
        if psdu.len() > usize::from(max_psdu_length) {
            return Err(Error::Operation(OpError::BufferAccessBeyondFrameFormat(
                psdu.len(),
                max_psdu_length,
            )));
        }

        self.device.read_rx_buffer(psdu)?;
        Ok(())
    }

    async fn transmit(
        &mut self,
        start_at: Instant,
        length: u16,
    ) -> Result<(), Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_data) = self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };
        let run_config = run_data.config;

        check_start_instant_alignment(start_at)?;

        let max_psdu_length = max_psdu_length(run_config.long_frame_format);
        if length > max_psdu_length {
            return Err(Error::Operation(OpError::TxLengthAboveFrameFormat(
                length,
                max_psdu_length,
            )));
        }

        if run_config.correct_tx_fcs && length < phy::FCS_LENGTH {
            return Err(Error::Operation(OpError::TxLengthLessThanFcs(length)));
        }

        let shr_duration = phy::shr_duration(run_config.prf, run_config.sfd_type, run_config.psr);
        let rmarker_at = start_at + shr_duration;

        self.device.set_tx_config(
            run_config.psr,
            run_config.bit_rate,
            run_config.ranging,
            length,
        )?;
        self.device.set_dx_time(rmarker_at)?;

        self.device.send_command(FastCommand::Dtx)?;
        let start_instant = self.device.get_sys_timestamp()?;
        let overtime = start_instant - start_at;

        let imm_events = self.device.get_events()?;
        if imm_events.contains(Events::HPDWARN) {
            self.device.send_command(FastCommand::Txrxoff)?;
            self.device.clear_all_events()?;
            return Err(OpError::StartInstantPassed(overtime).into());
        }

        let bug_state = self.device.is_tx_missed_deadline_state()?;
        if bug_state {
            self.device.send_command(FastCommand::Txrxoff)?;
            self.device.clear_all_events()?;
            return Err(OpError::StartInstantPassed(overtime).into());
        }

        const _START_DELAY_MAX: Duration = Timebase::PERIOD;
        let start_delay = start_at - start_instant;

        const _FRAME_DURATION_MAX: Duration = Timebase::PERIOD; // significant exaggeration
        let frame_duration = shr_duration
            + phy::phr_duration(run_config.bit_rate)
            + phy::psdu_duration(run_config.bit_rate, length);

        const _EVENT_TIMEOUT_US_MAX: u64 = _START_DELAY_MAX
            .add(_FRAME_DURATION_MAX)
            .div_ceil(HOST_MICROSECOND_MIN);
        let event_timeout_us = (start_delay + frame_duration).div_ceil(HOST_MICROSECOND_MIN);

        const _ASSERT: u64 = u32::MAX as u64 - _EVENT_TIMEOUT_US_MAX;
        let event_timeout_us = unwrap!(u32::try_from(event_timeout_us));

        self.device.set_event_mask(Events::TXFRS)?;
        if !self.device.wait_for_events(event_timeout_us).await? {
            self.device.send_command(FastCommand::Txrxoff)?;
            self.device.clear_all_events()?;
            return Err(Error::Device(DeviceError::TxStateTimeout {
                timeout_us: event_timeout_us,
            }));
        }

        self.device.clear_all_events()?;

        Ok(())
    }

    async fn receive(
        &mut self,
        start_at: Instant,
        max_preamble_hunt: Option<NonZeroU16>,
        rx_timeout: Duration,
    ) -> Result<Option<phy::RxReport<Timebase>>, Error<Self::IoError, Self::DevError>> {
        let InnerState::Running(run_data) = self.state else {
            return Err(Error::Operation(OpError::ProhibitedInCurrentState(
                self.state.into(),
            )));
        };

        check_start_instant_alignment(start_at)?;

        if rx_timeout > device::MAX_RX_TIMEOUT {
            return Err(Error::Operation(OpError::ExcessiveRxTimeout(rx_timeout)));
        }

        self.device
            .set_preamble_timeout(run_data.pac, max_preamble_hunt)?;
        self.device.set_dx_time(start_at)?;
        self.device.set_rx_frame_timeout(rx_timeout)?;

        self.device.send_command(FastCommand::Drx)?;
        let start_instant = self.device.get_sys_timestamp()?;
        let overtime = start_instant - start_at;

        let imm_events = self.device.get_events()?;
        if imm_events.contains(Events::HPDWARN) {
            self.device.send_command(FastCommand::Txrxoff)?;
            self.device.clear_all_events()?;
            return Err(OpError::StartInstantPassed(overtime).into());
        }

        self.device.set_event_mask(RX_TERMINATION_EVENTS)?;

        const _START_DELAY_MAX: Duration = Timebase::PERIOD;
        let start_delay = start_at - start_instant;

        const _EVENT_TIMEOUT_US_MAX: u64 = _START_DELAY_MAX
            .add(device::MAX_RX_TIMEOUT)
            .div_ceil(HOST_MICROSECOND_MIN);
        let event_timeout_us = (start_delay + rx_timeout).div_ceil(HOST_MICROSECOND_MIN);

        const _ASSERT: u64 = u32::MAX as u64 - _EVENT_TIMEOUT_US_MAX;
        let event_timeout_us = unwrap!(u32::try_from(event_timeout_us));

        if !self.device.wait_for_events(event_timeout_us).await? {
            self.device.send_command(FastCommand::Txrxoff)?;
            self.device.clear_all_events()?;
            return Err(Error::Device(DeviceError::RxStateTimeout {
                timeout_us: event_timeout_us,
            }));
        }
        let events = self.device.get_events()?;
        self.device.clear_all_events()?;

        // TODO: Add RX error signalling
        if !events.contains(Events::RXFR) {
            return Ok(None);
        }

        if run_data.config.ranging && !events.contains(Events::CIADONE) {
            return Ok(None);
        }

        let frame_length = self.device.get_rx_frame_length()?;
        let timestamp = if run_data.config.ranging {
            self.device.get_fine_rx_timestamp()?
        } else {
            self.device.get_coarse_rx_timestamp()?
        };

        Ok(Some(phy::RxReport {
            length: frame_length,
            fcs_good: events.contains(Events::RXFCG),
            timestamp,
        }))
    }
}
