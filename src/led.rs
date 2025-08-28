use rp_pico as bsp;

use crate::{
    button::ButtonDirection,
    channel::Receiver,
    time::{Duration, Ticker, Timer},
};
use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullDown, SioOutput};
use embedded_hal::digital::{OutputPin, StatefulOutputPin};

pub const NUM_LEDS: usize = 10;

pub type LedPin = Pin<DynPinId, FunctionSio<SioOutput>, PullDown>;
pub type LedRow = [LedPin; NUM_LEDS];

enum LedState<'a> {
    Toggle,
    Wait(Timer<'a>),
}

pub struct LedTask<'a> {
    leds: LedRow,
    active_led: usize,
    ticker: &'a Ticker,
    state: LedState<'a>,
    receiver: Receiver<'a, ButtonDirection>,
}

impl<'a> LedTask<'a> {
    pub fn new(leds: LedRow, ticker: &'a Ticker, receiver: Receiver<'a, ButtonDirection>) -> Self {
        Self {
            leds,
            active_led: 0,
            ticker,
            state: LedState::Toggle,
            receiver,
        }
    }

    fn shift(&mut self, direction: ButtonDirection) {
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

    pub fn poll(&mut self) {
        match self.state {
            LedState::Toggle => {
                self.leds[self.active_led].toggle().ok();
                self.state = LedState::Wait(Timer::new(Duration::millis(500), self.ticker));
            }
            LedState::Wait(ref timer) => {
                if timer.is_ready() {
                    self.state = LedState::Toggle;
                }
                if let Some(direction) = self.receiver.receive() {
                    self.shift(direction);
                    self.state = LedState::Toggle;
                }
            }
        }
    }
}
