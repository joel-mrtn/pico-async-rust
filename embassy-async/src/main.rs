#![no_std]
#![no_main]

mod button;
mod led;

use crate::led::LedRow;
use button::ButtonDirection;
use defmt::{info, panic};
use embassy_executor::Spawner;
use embassy_rp::gpio::{self, Input, Output};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::Timer;
use futures::{FutureExt, select_biased};

use {defmt_rtt as _, panic_probe as _};

static CHANNEL: Channel<ThreadModeRawMutex, ButtonDirection, 1> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());

    let button_l = Input::new(p.PIN_10, gpio::Pull::Up);
    let button_r = Input::new(p.PIN_11, gpio::Pull::Up);

    let leds = [
        Output::new(p.PIN_16, gpio::Level::Low),
        Output::new(p.PIN_17, gpio::Level::Low),
        Output::new(p.PIN_18, gpio::Level::Low),
        Output::new(p.PIN_19, gpio::Level::Low),
        Output::new(p.PIN_20, gpio::Level::Low),
        Output::new(p.PIN_21, gpio::Level::Low),
        Output::new(p.PIN_22, gpio::Level::Low),
        Output::new(p.PIN_26, gpio::Level::Low),
        Output::new(p.PIN_27, gpio::Level::Low),
        Output::new(p.PIN_28, gpio::Level::Low),
    ];

    spawner
        .spawn(button_task(button_l, ButtonDirection::Left))
        .unwrap();
    spawner
        .spawn(button_task(button_r, ButtonDirection::Right))
        .unwrap();

    let mut blinker = LedRow::new(leds);

    loop {
        blinker.toggle();
        select_biased! {
            direction = CHANNEL.receive().fuse() => {
                blinker.shift(direction);
            }
            _ = Timer::after_millis(500).fuse() => {}
        }
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn button_task(mut pin: Input<'static>, direction: ButtonDirection) {
    loop {
        pin.wait_for_low().await;
        CHANNEL.send(direction).await;
        Timer::after_millis(200).await;
        pin.wait_for_high().await;
    }
}
