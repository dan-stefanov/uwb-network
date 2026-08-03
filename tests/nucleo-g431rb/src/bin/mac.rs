#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m_rt::entry;
use defmt::*;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_executor::{Executor, InterruptExecutor};
use embassy_stm32::gpio;
use embassy_stm32::interrupt;
use embassy_stm32::interrupt::{InterruptExt, Priority};
use embassy_stm32::mode::Blocking;
use embassy_stm32::spi;
use embassy_stm32::{bind_interrupts, exti};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_time::{Delay, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use uwb_network_phy::Phy;
use uwb_network_phy_dw3000 as dw3000_phy;
use uwb_network_phy_dw3000::Dw3000Phy;
use uwb_network_ranging_mac as mac;

bind_interrupts!(
    struct Irqs {
        EXTI9_5 => exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI9_5>;
    }
);

struct DwPeripheral {
    irq: exti::ExtiInput<'static>,
    reset: gpio::OutputOpenDrain<'static>,
    cs: gpio::Output<'static>,
    spi: spi::Spi<'static, Blocking, spi::mode::Master>,
}

#[embassy_executor::task]
async fn run_mac_worker(dw_peripheral: DwPeripheral) {
    info!("Configure UWB module");
    let spi_bus = NoopMutex::new(RefCell::new(dw_peripheral.spi));
    let spi_dev = SpiDevice::new(&spi_bus, dw_peripheral.cs);

    let dw_interface = dw3000_phy::interface::SpiInterface::new(
        spi_dev,
        dw_peripheral.reset,
        dw_peripheral.irq,
        Delay,
    );

    let mut phy =
        unwrap!(Dw3000Phy::init(dw_interface, nucleo_g431rb::dw3000_device_config()).await);

    if cfg!(feature = "initiator") {
        mac::functional_tests::slotted_mac::initiator(&mut phy).await;
    } else {
        mac::functional_tests::slotted_mac::responder(&mut phy).await;
    }

    info!("Power off UWB module");
    unwrap!(phy.stop().await);

    info!("Test is finished");
    loop {
        Timer::after_millis(1000).await;
        info!("1 sec passed");
    }
}

#[embassy_executor::task]
async fn run_main_loop() {
    loop {
        Timer::after_millis(1000).await;
        info!("Main thread: 1 sec passed");
    }
}

static EXECUTOR_MAC: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_MAIN: StaticCell<Executor> = StaticCell::new();

#[interrupt]
unsafe fn UART4() {
    unsafe { EXECUTOR_MAC.on_interrupt() }
}

#[entry]
fn main() -> ! {
    info!("Hello World!");

    let p = {
        use embassy_stm32::rcc;
        // 64MHz for 32MHz SPI speed
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

    info!("Configure interface");
    let spi_config = {
        use embassy_stm32::time::Hertz;
        let mut config = spi::Config::default();
        config.mode = spi::MODE_0;
        config.bit_order = spi::BitOrder::MsbFirst;
        config.frequency = Hertz::mhz(32);
        config
    };

    let dw_peripherals = DwPeripheral {
        irq: exti::ExtiInput::new(p.PA9, p.EXTI9, gpio::Pull::Down, Irqs),
        reset: gpio::OutputOpenDrain::new(p.PA8, gpio::Level::High, gpio::Speed::Low),
        cs: gpio::Output::new(p.PB6, gpio::Level::High, gpio::Speed::High),
        spi: spi::Spi::new_blocking(p.SPI1, p.PA5, p.PA7, p.PA6, spi_config),
    };

    interrupt::UART4.set_priority(Priority::P6);
    let spawner = EXECUTOR_MAC.start(interrupt::UART4);
    unwrap!(spawner.spawn(run_mac_worker(dw_peripherals)));

    let executor = EXECUTOR_MAIN.init(Executor::new());
    executor.run(|spawner| {
        unwrap!(spawner.spawn(run_main_loop()));
    });
}
