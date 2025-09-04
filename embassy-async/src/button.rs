use defmt::Format;
use defmt_rtt as _;

#[derive(Clone, Copy, Debug, Format)]
pub enum ButtonDirection {
    Left,
    Right,
}
