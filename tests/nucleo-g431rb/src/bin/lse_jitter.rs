#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

use embassy_stm32::gpio;
use embassy_stm32::time::Hertz;
use nucleo_g431rb::lse_capture::LseCapture;
use nucleo_g431rb::stat::Stats;

const PERIODS: usize = 1000;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = {
        use embassy_stm32::rcc;
        let mut config = embassy_stm32::Config::default();
        config.rcc.hse = Some(rcc::Hse {
            freq: Hertz::mhz(24),
            mode: rcc::HseMode::Oscillator,
        });
        config.rcc.sys = rcc::Sysclk::HSE;
        config.rcc.ls.lse = Some(rcc::LseConfig {
            frequency: Hertz::hz(32768),
            mode: rcc::LseMode::Oscillator(rcc::LseDrive::Low),
        });
        embassy_stm32::init(config)
    };
    info!("Hello World!");

    // Set UWB chip to reset
    let _reset_pin = gpio::OutputOpenDrain::new(p.PA8, gpio::Level::Low, gpio::Speed::Low);
    let _cs_pin = gpio::Output::new(p.PB6, gpio::Level::High, gpio::Speed::High);

    let mut lse_capture = LseCapture::new(p.TIM16, Hertz::mhz(24));

    info!("Measuring LSE period over {} cycles...", PERIODS);

    loop {
        // Wait for first capture to establish baseline
        let mut last_capture = loop {
            if let Some(val) = lse_capture.pop() {
                break val;
            }
        };

        let mut stats = Stats::new();

        for _ in 0..PERIODS {
            let capture = loop {
                if let Some(val) = lse_capture.pop() {
                    break val;
                }
            };

            // Handle 16-bit timer wraparound
            let period = capture.wrapping_sub(last_capture);
            stats.update(period as f32);
            last_capture = capture;
        }

        info!("Period statistic: {}", stats);
        Timer::after_millis(1000).await;
    }
}
