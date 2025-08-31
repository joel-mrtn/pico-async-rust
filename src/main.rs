#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_halt as _;

use rp_pico as bsp;

// mod button;
// mod channel;
mod executor;
mod led;
mod time;

use bsp::entry;
use bsp::hal::{Watchdog, clocks::init_clocks_and_plls, pac, sio};
use core::pin::pin;
use defmt::info;
use time::Ticker;

// use crate::button::ButtonDirection;
// use crate::channel::{Channel, Receiver};
use crate::led::{LedPin, LedRow, NUM_LEDS};
use crate::time::Duration;

#[entry]
fn main() -> ! {
    info!("Starting...");
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

    let leds: [LedPin; NUM_LEDS] = [
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

    Ticker::init(pac.TIMER, &mut pac.RESETS, &clocks);

    let led_task = pin!(led_task(leds));

    executor::run_tasks(&mut [led_task]);
}

async fn led_task(leds: [LedPin; NUM_LEDS]) {
    let mut blinker = LedRow::new(leds);
    loop {
        blinker.toggle();
        time::delay(Duration::millis(500)).await
    }
}
