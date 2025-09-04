use rp_pico as bsp;

use crate::button::ButtonDirection;
use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullDown, SioOutput};
use defmt::info;
use embedded_hal::digital::{OutputPin, StatefulOutputPin};

pub const NUM_LEDS: usize = 10;

pub type LedPin = Pin<DynPinId, FunctionSio<SioOutput>, PullDown>;

pub struct LedRow {
    leds: [LedPin; NUM_LEDS],
    active_led: usize,
}

impl LedRow {
    pub fn new(leds: [LedPin; NUM_LEDS]) -> Self {
        Self {
            leds,
            active_led: 0,
        }
    }

    pub fn shift(&mut self, direction: ButtonDirection) {
        info!("LED ROW: shifting led to direction {}", direction);
        self.leds[self.active_led].set_low().ok();
        self.active_led = match direction {
            ButtonDirection::Left => match self.active_led {
                0 => NUM_LEDS - 1,
                _ => self.active_led - 1,
            },
            ButtonDirection::Right => (self.active_led + 1) % NUM_LEDS,
        };
        self.leds[self.active_led].set_low().ok();
    }

    pub fn toggle(&mut self) {
        info!("LED ROW: toggling led {}", self.active_led);
        self.leds[self.active_led].toggle().ok();
    }
}
