#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m_rt::entry;
use defmt::*;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice as SpiDeviceImpl;
use embassy_stm32::{gpio, spi};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_time::Delay;
use embassy_time::Instant;
use embedded_hal::delay::DelayNs;
use embedded_hal::spi::{SpiBus, SpiDevice};
use {defmt_rtt as _, panic_probe as _};

#[entry]
fn main() -> ! {
    let mut p = {
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
        use embassy_stm32::time::Hertz;
        let mut config = spi::Config::default();
        config.mode = spi::MODE_0;
        config.bit_order = spi::BitOrder::MsbFirst;
        config.frequency = Hertz::mhz(32);
        config
    };

    let mut delay = Delay {};

    loop {
        let mut spi = spi::Spi::new_blocking(
            p.SPI1.reborrow(),
            p.PA5.reborrow(),
            p.PA7.reborrow(),
            p.PA6.reborrow(),
            spi_config,
        );
        test_bus(&mut spi);

        let spi_bus = NoopMutex::new(RefCell::new(spi));
        let mut spi_dev = SpiDeviceImpl::new(&spi_bus, &mut cs_pin);
        test_device(&mut spi_dev);

        delay.delay_ms(1000);
    }
}

const TEST_SIZE: usize = 127;
const ITER_COUNT: usize = 10;

fn test_bus(mut spi: impl SpiBus) {
    let mut buf = [0u8; TEST_SIZE];
    let buf2 = [0u8; TEST_SIZE];

    let write_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.write(&buf).unwrap();
    }
    let write_end = Instant::now();
    let write_duration = write_end - write_begin;

    let read_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.write(&buf).unwrap();
    }
    let read_end = Instant::now();
    let read_duration = read_end - read_begin;

    let transfer_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.transfer(&mut buf, &buf2).unwrap();
    }
    let transfer_end = Instant::now();
    let transfer_duration = transfer_end - transfer_begin;

    let transfer_in_place_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.transfer_in_place(&mut buf).unwrap();
    }
    let transfer_in_place_end = Instant::now();
    let transfer_in_place_duration = transfer_in_place_end - transfer_in_place_begin;

    info!(
        "Block size {} x {}: write {}us ({}), read: {}us ({}), transfer: {}us ({}), transfer_in_place: {}us ({})",
        TEST_SIZE,
        ITER_COUNT,
        write_duration.as_micros(),
        write_duration.as_ticks(),
        read_duration.as_micros(),
        read_duration.as_ticks(),
        transfer_duration.as_micros(),
        transfer_duration.as_ticks(),
        transfer_in_place_duration.as_micros(),
        transfer_in_place_duration.as_ticks(),
    );
}

fn test_device(mut spi: impl SpiDevice) {
    let mut buf = [0u8; TEST_SIZE];
    let buf2 = [0u8; TEST_SIZE];

    let write_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.write(&buf).unwrap();
    }
    let write_end = Instant::now();
    let write_duration = write_end - write_begin;

    let read_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.write(&buf).unwrap();
    }
    let read_end = Instant::now();
    let read_duration = read_end - read_begin;

    let transfer_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.transfer(&mut buf, &buf2).unwrap();
    }
    let transfer_end = Instant::now();
    let transfer_duration = transfer_end - transfer_begin;

    let transfer_in_place_begin = Instant::now();
    for _ in 0..ITER_COUNT {
        spi.transfer_in_place(&mut buf).unwrap();
    }
    let transfer_in_place_end = Instant::now();
    let transfer_in_place_duration = transfer_in_place_end - transfer_in_place_begin;

    info!(
        "Block size {} x {}: write {}us ({}), read: {}us ({}), transfer: {}us ({}), transfer_in_place: {}us ({})",
        TEST_SIZE,
        ITER_COUNT,
        write_duration.as_micros(),
        write_duration.as_ticks(),
        read_duration.as_micros(),
        read_duration.as_ticks(),
        transfer_duration.as_micros(),
        transfer_duration.as_ticks(),
        transfer_in_place_duration.as_micros(),
        transfer_in_place_duration.as_ticks(),
    );
}
