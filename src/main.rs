#![no_std]
#![no_main]

use panic_halt as _;

use rp_pico as bsp;

mod button;
mod channel;
mod led;
mod time;

use bsp::entry;
use bsp::hal::{Watchdog, clocks::init_clocks_and_plls, pac, sio};
use time::Ticker;

use crate::button::{ButtonDirection, ButtonTask};
use crate::channel::Channel;
use crate::led::{LedRow, LedTask};

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().expect("Peripherals already taken");
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = sio::Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let led_pins: LedRow = [
        pins.gpio16.into_push_pull_output().into_dyn_pin(),
        pins.gpio17.into_push_pull_output().into_dyn_pin(),
        pins.gpio18.into_push_pull_output().into_dyn_pin(),
        pins.gpio19.into_push_pull_output().into_dyn_pin(),
        pins.gpio20.into_push_pull_output().into_dyn_pin(),
        pins.gpio21.into_push_pull_output().into_dyn_pin(),
        pins.gpio22.into_push_pull_output().into_dyn_pin(),
        pins.gpio26.into_push_pull_output().into_dyn_pin(),
        pins.gpio27.into_push_pull_output().into_dyn_pin(),
        pins.gpio28.into_push_pull_output().into_dyn_pin(),
    ];

    let button_l = pins.gpio10.into_pull_up_input().into_dyn_pin();
    let button_r = pins.gpio11.into_pull_up_input().into_dyn_pin();

    let ticker = Ticker::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let channel: Channel<ButtonDirection> = Channel::new();
    let mut led_task = LedTask::new(led_pins, &ticker, channel.get_receiver());
    let mut button_l_task = ButtonTask::new(
        button_l,
        &ticker,
        button::ButtonDirection::Left,
        channel.get_sender(),
    );
    let mut button_r_task = ButtonTask::new(
        button_r,
        &ticker,
        button::ButtonDirection::Right,
        channel.get_sender(),
    );

    loop {
        led_task.poll();
        button_l_task.poll();
        button_r_task.poll();
    }
}
