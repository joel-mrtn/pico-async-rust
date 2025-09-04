#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use rp_pico as bsp;

mod button;
mod channel;
mod executor;
mod gpio;
mod led;
mod time;

use bsp::entry;
use bsp::hal::{Watchdog, clocks::init_clocks_and_plls, pac, sio};
use core::pin::pin;
use defmt::{debug, info};
use embedded_hal::digital::PinState;
use futures::{FutureExt, select_biased};
use time::Ticker;

use crate::button::{ButtonDirection, ButtonPin};
use crate::channel::{Channel, Receiver, Sender};
use crate::gpio::InputChannel;
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

    Ticker::init(pac.TIMER, &mut pac.RESETS, &clocks);

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

    let button_l = pins.gpio10.into_pull_up_input().into_dyn_pin();
    let button_r = pins.gpio11.into_pull_up_input().into_dyn_pin();

    let channel: Channel<ButtonDirection> = Channel::new();
    let led_task = pin!(led_task(leds, channel.get_receiver()));
    let button_l_task = pin!(button_task(
        button_l,
        ButtonDirection::Left,
        channel.get_sender(),
    ));
    let button_r_task = pin!(button_task(
        button_r,
        ButtonDirection::Right,
        channel.get_sender(),
    ));

    debug!("Initialization complete, run tasks...");
    executor::run_tasks(&mut [led_task, button_l_task, button_r_task]);
}

async fn led_task(leds: [LedPin; NUM_LEDS], mut receiver: Receiver<'_, ButtonDirection>) {
    debug!("LED TASK: called!");
    let mut blinker = LedRow::new(leds);
    loop {
        debug!("LED TASK: toggle led");
        blinker.toggle();
        select_biased! {
            direction = receiver.receive().fuse() => {
                debug!("LED TASK: shift led");
                blinker.shift(direction);
            }
            _ = time::delay(Duration::millis(500)).fuse() => {}
        }
    }
}

async fn button_task(
    pin: ButtonPin,
    direction: ButtonDirection,
    sender: Sender<'_, ButtonDirection>,
) {
    debug!("BUTTON TASK {}: called!", direction);
    let mut input = InputChannel::new(pin);
    loop {
        debug!("BUTTON TASK {}: wait for input...", direction);
        input.wait_for(PinState::Low).await;
        debug!("BUTTON TASK {}: send direction", direction);
        sender.send(direction);
        debug!("BUTTON TASK {}: debounce delay", direction);
        time::delay(Duration::millis(150)).await;
        debug!("BUTTON TASK {}: wait for high pin state...", direction);
        input.wait_for(PinState::High).await;
    }
}
