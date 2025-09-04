use defmt::info;
use embassy_rp::gpio::Output;

use crate::button::ButtonDirection;

const NUM_LEDS: usize = 10;

pub struct LedRow {
    leds: [Output<'static>; NUM_LEDS],
    active_led: usize,
}

impl LedRow {
    pub fn new(leds: [Output<'static>; NUM_LEDS]) -> Self {
        Self {
            leds,
            active_led: 0,
        }
    }

    pub fn shift(&mut self, direction: ButtonDirection) {
        info!("Button press detected...");
        self.leds[self.active_led].set_low();
        self.active_led = match direction {
            ButtonDirection::Left => match self.active_led {
                0 => NUM_LEDS - 1,
                _ => self.active_led - 1,
            },
            ButtonDirection::Right => (self.active_led + 1) % NUM_LEDS,
        };
        self.leds[self.active_led].set_low();
    }

    pub fn toggle(&mut self) {
        info!("Blinking LED {}", self.active_led);
        self.leds[self.active_led].toggle();
    }
}
