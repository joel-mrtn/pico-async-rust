use rp_pico as bsp;

use bsp::hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};
use defmt::Format;

pub type ButtonPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;

#[derive(Clone, Copy, Debug, Format)]
pub enum ButtonDirection {
    Left,
    Right,
}
