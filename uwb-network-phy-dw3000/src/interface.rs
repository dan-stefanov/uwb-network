use core::fmt;
use embassy_futures::select::{Either, select};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::{Operation, SpiDevice};
use embedded_hal_async::{delay::DelayNs, digital::Wait};

pub const MAX_FILE_ID: u8 = 0x1f;
pub const MAX_COMMAND: u8 = 0x1f;
pub const MAX_OFFSET: u8 = 0x7f;

/// Check and convert file ID for easy integration to header
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct FileId(u8);
impl FileId {
    pub const fn new(file_id: u8) -> Self {
        core::assert!(file_id <= MAX_FILE_ID);
        Self(file_id)
    }
}

impl From<FileId> for u8 {
    fn from(value: FileId) -> Self {
        value.0
    }
}

/// Check and combine file ID, offset for easy integration to header
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct FullAddress(u16);
impl FullAddress {
    pub const fn new(file_id: FileId, offset: u8) -> Self {
        core::assert!(offset <= MAX_OFFSET);
        Self((file_id.0 as u16) << 9 | (offset as u16) << 2)
    }
}

mod headers {
    pub const SHORT_COMMAND: u8 = 0b1000_0001;
    pub const SHORT_READ: u8 = 0b0000_0000;
    pub const SHORT_WRITE: u8 = 0b1000_0000;
    pub const FULL_READ: u16 = 0b0100_0000_0000_0000;
    pub const FULL_WRITE: u16 = 0b1100_0000_0000_0000;
    pub const FULL_MODIFY_1: u16 = 0b1100_0000_0000_0001;
    pub const FULL_MODIFY_2: u16 = 0b1100_0000_0000_0010;
    pub const FULL_MODIFY_4: u16 = 0b1100_0000_0000_0011;
}

#[allow(async_fn_in_trait)]
pub trait Interface {
    type Error: fmt::Debug;

    fn set_reset(&mut self) -> Result<(), Self::Error>;
    fn clear_reset(&mut self) -> Result<(), Self::Error>;
    fn is_irq(&mut self) -> Result<bool, Self::Error>;
    async fn wait_for_irq(&mut self) -> Result<(), Self::Error>;
    async fn wait_for_irq_with_timeout(&mut self, timeout_us: u32) -> Result<bool, Self::Error>;
    async fn delay_us(&mut self, delay_us: u32);

    fn wake_up(&mut self) -> Result<(), Self::Error>;
    fn send_command(&mut self, command: u8) -> Result<(), Self::Error>;
    fn read_fast(&mut self, file_id: FileId, data: &mut [u8]) -> Result<(), Self::Error>;
    fn write_fast(&mut self, file_id: FileId, data: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, addr: FullAddress, data: &mut [u8]) -> Result<(), Self::Error>;
    fn write(&mut self, addr: FullAddress, data: &[u8]) -> Result<(), Self::Error>;

    fn read_register(&mut self, addr: FullAddress, length: usize) -> Result<u64, Self::Error>;
    fn write_register(
        &mut self,
        addr: FullAddress,
        value: u64,
        length: usize,
    ) -> Result<(), Self::Error>;

    // TODO: generalize over primitive type
    fn modify_1(&mut self, addr: FullAddress, and_mask: u8, or_mask: u8)
    -> Result<(), Self::Error>;

    fn modify_2(
        &mut self,
        addr: FullAddress,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<(), Self::Error>;

    fn modify_4(
        &mut self,
        addr: FullAddress,
        and_mask: u32,
        or_mask: u32,
    ) -> Result<(), Self::Error>;
}

pub enum SpiInterfaceError<SPI: SpiDevice, RST: OutputPin, IRQ: InputPin> {
    Spi(SPI::Error),
    ResetPin(RST::Error),
    IrqPin(IRQ::Error),
}

// The derived Debug requires SPI, RST, IRQ to implement Debug as well,
// though only an associated type is actually used.
impl<SPI: SpiDevice, RST: OutputPin, IRQ: InputPin> fmt::Debug
    for SpiInterfaceError<SPI, RST, IRQ>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpiInterfaceError::Spi(spi) => write!(f, "Error::Spi({:?})", spi),
            SpiInterfaceError::ResetPin(spi) => write!(f, "Error::ResetPin({:?})", spi),
            SpiInterfaceError::IrqPin(spi) => write!(f, "Error::IrqPin({:?})", spi),
        }
    }
}

#[cfg(feature = "defmt")]
impl<SPI, RST, IRQ> defmt::Format for SpiInterfaceError<SPI, RST, IRQ>
where
    SPI: SpiDevice,
    SPI::Error: defmt::Format,
    RST: OutputPin,
    RST::Error: defmt::Format,
    IRQ: InputPin,
    IRQ::Error: defmt::Format,
{
    fn format(&self, f: defmt::Formatter) {
        match self {
            SpiInterfaceError::Spi(error) => defmt::write!(f, "Spi({:?})", error),
            SpiInterfaceError::ResetPin(error) => defmt::write!(f, "ResetPin({:?})", error),
            SpiInterfaceError::IrqPin(error) => defmt::write!(f, "IrqPin({:?})", error),
        }
    }
}

const CS_WAKE_UP_DURATION_NS: u32 = 500_000;

pub struct SpiInterface<SPI, RST, IRQ, DELAY> {
    spi_dev: SPI,
    reset_pin: RST,
    irq_pin: IRQ,
    delay: DELAY,
}

impl<SPI: SpiDevice, RST: OutputPin, IRQ: InputPin + Wait, DELAY: DelayNs>
    SpiInterface<SPI, RST, IRQ, DELAY>
{
    /// Create chip interface
    ///
    /// Device pulls reset pin low during initialization, configure it as open-collector
    pub fn new(spi: SPI, reset: RST, irq: IRQ, delay: DELAY) -> Self {
        Self {
            spi_dev: spi,
            reset_pin: reset,
            irq_pin: irq,
            delay,
        }
    }
}

impl<SPI: SpiDevice, RST: OutputPin, IRQ: InputPin + Wait, DELAY: DelayNs> Interface
    for SpiInterface<SPI, RST, IRQ, DELAY>
{
    type Error = SpiInterfaceError<SPI, RST, IRQ>;

    fn set_reset(&mut self) -> Result<(), Self::Error> {
        self.reset_pin
            .set_low()
            .map_err(SpiInterfaceError::ResetPin)
    }

    fn clear_reset(&mut self) -> Result<(), Self::Error> {
        self.reset_pin
            .set_high()
            .map_err(SpiInterfaceError::ResetPin)
    }

    fn is_irq(&mut self) -> Result<bool, Self::Error> {
        // IRQ is active high by default, driver should not change it
        self.irq_pin.is_high().map_err(SpiInterfaceError::IrqPin)
    }

    async fn wait_for_irq(&mut self) -> Result<(), Self::Error> {
        // IRQ is active high by default, driver should not change it
        self.irq_pin
            .wait_for_high()
            .await
            .map_err(SpiInterfaceError::IrqPin)
    }

    async fn wait_for_irq_with_timeout(&mut self, timeout_us: u32) -> Result<bool, Self::Error> {
        match select(
            self.irq_pin.wait_for_high(),
            self.delay.delay_us(timeout_us),
        )
        .await
        {
            Either::First(result) => result.map(|_| true).map_err(SpiInterfaceError::IrqPin),
            Either::Second(()) => Ok(false),
        }
    }

    async fn delay_us(&mut self, delay_us: u32) {
        self.delay.delay_us(delay_us).await;
    }

    fn wake_up(&mut self) -> Result<(), Self::Error> {
        self.spi_dev
            .transaction(&mut [Operation::DelayNs(CS_WAKE_UP_DURATION_NS)])
            .map_err(SpiInterfaceError::Spi)
    }

    fn send_command(&mut self, command: u8) -> Result<(), Self::Error> {
        assert!(command <= MAX_COMMAND);
        let header = (headers::SHORT_COMMAND | command << 1).to_be_bytes();
        self.spi_dev.write(&header).map_err(SpiInterfaceError::Spi)
    }

    fn read_fast(&mut self, file_id: FileId, data: &mut [u8]) -> Result<(), Self::Error> {
        let header = (headers::SHORT_READ | file_id.0 << 1).to_be_bytes();
        self.spi_dev
            .transaction(&mut [Operation::Write(&header), Operation::Read(data)])
            .map_err(SpiInterfaceError::Spi)
    }

    fn write_fast(&mut self, file_id: FileId, data: &[u8]) -> Result<(), Self::Error> {
        let header = (headers::SHORT_WRITE | file_id.0 << 1).to_be_bytes();
        self.spi_dev
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
            .map_err(SpiInterfaceError::Spi)
    }

    fn read(&mut self, addr: FullAddress, data: &mut [u8]) -> Result<(), Self::Error> {
        let header = (headers::FULL_READ | addr.0).to_be_bytes();

        self.spi_dev
            .transaction(&mut [Operation::Write(&header), Operation::Read(data)])
            .map_err(SpiInterfaceError::Spi)
    }

    fn write(&mut self, addr: FullAddress, data: &[u8]) -> Result<(), Self::Error> {
        let header = (headers::FULL_WRITE | addr.0).to_be_bytes();

        self.spi_dev
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
            .map_err(SpiInterfaceError::Spi)
    }

    fn read_register(&mut self, addr: FullAddress, length: usize) -> Result<u64, Self::Error> {
        assert!(length <= 8);
        let header = (headers::FULL_READ | addr.0).to_be_bytes();
        let mut buf = [0u8; 10];
        buf[..header.len()].copy_from_slice(&header);

        self.spi_dev
            .transfer_in_place(&mut buf[..header.len() + length])
            .map_err(SpiInterfaceError::Spi)?;
        Ok(u64::from_le_bytes(unwrap!(buf[header.len()..].try_into())))
    }

    fn write_register(
        &mut self,
        addr: FullAddress,
        value: u64,
        length: usize,
    ) -> Result<(), Self::Error> {
        assert!(length <= 8);
        let header = (headers::FULL_WRITE | addr.0).to_be_bytes();
        let mut buf = [0u8; 10];
        buf[..header.len()].copy_from_slice(&header);
        buf[header.len()..].copy_from_slice(&value.to_le_bytes());

        self.spi_dev
            .write(&buf[..header.len() + length])
            .map_err(SpiInterfaceError::Spi)
    }

    fn modify_1(
        &mut self,
        addr: FullAddress,
        and_mask: u8,
        or_mask: u8,
    ) -> Result<(), Self::Error> {
        let header = (headers::FULL_MODIFY_1 | addr.0).to_be_bytes();
        let mut buf = [0u8; 4];
        buf[0..2].copy_from_slice(&header);
        buf[2] = and_mask;
        buf[3] = or_mask;
        self.spi_dev.write(&buf).map_err(SpiInterfaceError::Spi)
    }

    fn modify_2(
        &mut self,
        addr: FullAddress,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<(), Self::Error> {
        let header = (headers::FULL_MODIFY_2 | addr.0).to_be_bytes();
        let mut buf = [0u8; 6];
        buf[0..2].copy_from_slice(&header);
        buf[2..4].copy_from_slice(&and_mask.to_le_bytes());
        buf[4..6].copy_from_slice(&or_mask.to_le_bytes());
        self.spi_dev.write(&buf).map_err(SpiInterfaceError::Spi)
    }

    fn modify_4(
        &mut self,
        addr: FullAddress,
        and_mask: u32,
        or_mask: u32,
    ) -> Result<(), Self::Error> {
        let header = (headers::FULL_MODIFY_4 | addr.0).to_be_bytes();
        let mut buf = [0u8; 10];
        buf[0..2].copy_from_slice(&header);
        buf[2..6].copy_from_slice(&and_mask.to_le_bytes());
        buf[6..10].copy_from_slice(&or_mask.to_le_bytes());
        self.spi_dev.write(&buf).map_err(SpiInterfaceError::Spi)
    }
}
