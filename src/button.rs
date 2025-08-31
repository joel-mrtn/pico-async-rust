use rp_pico as bsp;

use crate::{
    channel::Sender,
    time::{Duration, Ticker, Timer},
};
use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use embedded_hal::digital::InputPin;

pub type ButtonPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;

#[derive(Clone, Copy)]
pub enum ButtonDirection {
    Left,
    Right,
}

enum ButtonState {
    WaitForPress,
    Debounce(Timer),
}

pub struct ButtonTask<'a> {
    pin: ButtonPin,
    ticker: &'a Ticker,
    direction: ButtonDirection,
    state: ButtonState<'a>,
    sender: Sender<'a, ButtonDirection>,
}

impl<'a> ButtonTask<'a> {
    pub fn new(
        pin: ButtonPin,
        ticker: &'a Ticker,
        direction: ButtonDirection,
        sender: Sender<'a, ButtonDirection>,
    ) -> Self {
        Self {
            pin,
            ticker,
            direction,
            state: ButtonState::WaitForPress,
            sender,
        }
    }

    pub fn poll(&mut self) {
        match self.state {
            ButtonState::WaitForPress => {
                if self.pin.is_low().unwrap() {
                    self.sender.send(self.direction);
                    self.state =
                        ButtonState::Debounce(Timer::new(Duration::millis(200), self.ticker));
                }
            }
            ButtonState::Debounce(ref timer) => {
                if timer.is_ready() && self.pin.is_high().unwrap() {
                    self.state = ButtonState::WaitForPress;
                }
            }
        }
    }
}
