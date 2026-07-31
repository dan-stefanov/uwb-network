#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Instant, Timer};
use {defmt_rtt as _, panic_probe as _};

use embassy_stm32::time::Hertz;
use embassy_stm32::{gpio, spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal_async::spi::{SpiBus, SpiDevice};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = {
        use embassy_stm32::rcc;
        // 144MHz for 36MHz SPI speed
        let pll = rcc::Pll {
            source: rcc::PllSource::HSI,
            prediv: rcc::PllPreDiv::DIV1,
            mul: rcc::PllMul::MUL18,
            divp: None,
            divq: None,
            divr: Some(rcc::PllRDiv::DIV2),
        };

        let mut config = embassy_stm32::Config::default();
        config.rcc.sys = rcc::Sysclk::PLL1_R;
        config.rcc.pll = Some(pll);
        embassy_stm32::init(config)
    };
    info!("Hello World!");

    info!("Configure interface");
    // Set UWB chip to reset
    let _reset_pin = gpio::OutputOpenDrain::new(p.PA8, gpio::Level::Low, gpio::Speed::Low);

    let mut cs_pin = gpio::Output::new(p.PB6, gpio::Level::High, gpio::Speed::High);

    let spi_config = {
        let mut config = spi::Config::default();
        config.mode = spi::MODE_0;
        config.bit_order = spi::BitOrder::MsbFirst;
        config.frequency = Hertz::mhz(36);
        config
    };

    let mut spi = spi::Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6, p.DMA1_CH1, p.DMA1_CH2, spi_config,
    );

    loop {
        info!("Stable");
        test_bus(&mut spi).await;

        let spi_bus = Mutex::<NoopRawMutex, _>::new(&mut spi);
        let mut spi_dev =
            embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice::new(&spi_bus, &mut cs_pin);
        test_device(&mut spi_dev).await;

        Timer::after_millis(1000).await;
    }
}

const TEST_SIZE: usize = 6;

async fn test_bus(mut spi: impl SpiBus) {
    let buf = [0u8; TEST_SIZE];

    let before = Instant::now();
    for _ in 0..10 {
        spi.write(&buf).await.unwrap();
    }
    let after = Instant::now();

    info!(
        "Async send duration: {}us, {} ticks",
        (after - before).as_micros(),
        after - before
    );
}

async fn test_device(mut spi: impl SpiDevice) {
    let buf = [0u8; TEST_SIZE];

    let before = Instant::now();
    for _ in 0..10 {
        spi.write(&buf).await.unwrap();
    }
    let after = Instant::now();

    info!(
        "Async send duration: {}us, {} ticks",
        (after - before).as_micros(),
        after - before
    );
}
