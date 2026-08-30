#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::*;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti;
use embassy_stm32::gpio::{Level, Output, OutputOpenDrain, Pull, Speed};
use embassy_stm32::spi;
use embassy_stm32::time::Hertz;
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_time::Delay;
use {defmt_rtt as _, panic_probe as _};

use uwb_network_phy_dw3000 as dw3000_phy;
use uwb_network_phy_dw3000::Dw3000Phy;

bind_interrupts!(
    struct Irqs {
        EXTI9_5 => exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI9_5>;
    }
);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = {
        use embassy_stm32::rcc;

        let pll = rcc::Pll {
            source: rcc::PllSource::HSI,
            prediv: rcc::PllPreDiv::DIV1,
            mul: rcc::PllMul::MUL8,
            divp: None,
            divq: None,
            divr: Some(rcc::PllRDiv::DIV2),
        };

        let mut config = embassy_stm32::Config::default();
        config.rcc.pll = Some(pll);
        config.rcc.sys = rcc::Sysclk::PLL1_R;
        embassy_stm32::init(config)
    };

    let reset_pin = OutputOpenDrain::new(p.PA8, Level::High, Speed::Low);
    let irq = exti::ExtiInput::new(p.PA9, p.EXTI9, Pull::Down, Irqs);

    let mut spi_config = spi::Config::default();
    spi_config.mode = spi::MODE_0;
    spi_config.bit_order = spi::BitOrder::MsbFirst;
    spi_config.frequency = Hertz::mhz(32);

    let spi = spi::Spi::new_blocking(p.SPI1, p.PA5, p.PA7, p.PA6, spi_config);
    let spi_bus = NoopMutex::new(RefCell::new(spi));
    let cs_dev = Output::new(p.PB6, Level::High, Speed::High);
    let spi_dev = SpiDevice::new(&spi_bus, cs_dev);

    let interface = dw3000_phy::interface::SpiInterface::new(spi_dev, reset_pin, irq, Delay);
    let mut phy = unwrap!(Dw3000Phy::init(interface, nucleo_g431rb::dw3000_device_config()).await);

    if cfg!(feature = "initiator") {
        test_suite::timestamp_stat::initiator(&mut phy, nucleo_g431rb::UWB_CHANNEL).await;
    } else {
        test_suite::timestamp_stat::responder(&mut phy, nucleo_g431rb::UWB_CHANNEL).await;
    }
}
