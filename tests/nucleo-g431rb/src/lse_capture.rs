use embassy_stm32::time::Hertz;
use embassy_stm32::{Peri, peripherals, timer};

pub struct LseCapture {
    timer: timer::low_level::Timer<'static, peripherals::TIM16>,
}

impl LseCapture {
    pub fn new(tim: Peri<'static, peripherals::TIM16>, tick_freq: Hertz) -> Self {
        let mut timer = timer::low_level::Timer::new(tim);
        timer.set_counting_mode(timer::low_level::CountingMode::EdgeAlignedUp);
        timer.set_tick_freq(tick_freq);
        timer.generate_update_event();
        timer.start();

        // sets the channel to the input mode as well
        timer.set_input_ti_selection(
            timer::Channel::Ch1,
            timer::low_level::InputTISelection::Normal,
        );
        timer.set_input_capture_filter(
            timer::Channel::Ch1,
            timer::low_level::FilterValue::NO_FILTER,
        );
        timer.set_input_capture_mode(
            timer::Channel::Ch1,
            timer::low_level::InputCaptureMode::Rising,
        );
        timer.set_input_capture_prescaler(timer::Channel::Ch1, 0);
        timer.regs_gp16().tisel().modify(|w| {
            // tim16_ti1_in5 is lse_css_out
            // tim16_ti1_in6 is lsi
            // RM0440, table 77, p. 392
            w.set_tisel(0, 5);
        });

        timer.enable_channel(timer::Channel::Ch1, true);

        Self { timer }
    }

    pub fn pop(&mut self) -> Option<u16> {
        if self.timer.get_input_interrupt(timer::Channel::Ch1) {
            Some(self.timer.get_compare_value(timer::Channel::Ch1))
        } else {
            None
        }
    }
}
