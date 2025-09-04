use rp_pico as bsp;

use bsp::hal::{
    gpio::Interrupt::{EdgeHigh, EdgeLow},
    pac::{self, interrupt},
};
use core::{
    future::poll_fn,
    sync::atomic::{AtomicUsize, Ordering},
    task::Poll,
};
use defmt::{debug, info};
use embedded_hal::digital::{InputPin, PinState};

use crate::button::ButtonPin;
use crate::executor::{ExtWaker, wake_task};

const MAX_BUTTONS: usize = 2;
const INVALID_TASK_ID: usize = usize::MAX;
const INVALID_GPIO: usize = usize::MAX;

static WAKE_TASKS: [AtomicUsize; MAX_BUTTONS] = [
    AtomicUsize::new(INVALID_TASK_ID),
    AtomicUsize::new(INVALID_TASK_ID),
];

static GPIO_PINS: [AtomicUsize; MAX_BUTTONS] = [
    AtomicUsize::new(INVALID_GPIO),
    AtomicUsize::new(INVALID_GPIO),
];

pub struct InputChannel {
    pin: ButtonPin,
    index: usize,
}

impl InputChannel {
    pub fn new(pin: ButtonPin) -> Self {
        let index = GPIO_PINS
            .iter()
            .position(|pin| pin.load(Ordering::Relaxed) == INVALID_GPIO);
        let index = match index {
            Some(i) => i,
            None => panic!("No available slots for new InputChannel"),
        };

        GPIO_PINS[index].store(pin.id().num as usize, Ordering::Relaxed);

        pin.set_interrupt_enabled(EdgeLow, true);
        pin.set_interrupt_enabled(EdgeHigh, true);

        unsafe { pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0) }

        Self { pin, index }
    }

    pub async fn wait_for(&mut self, ready_state: PinState) {
        poll_fn(|cx| {
            let current_state = if self.pin.is_low().unwrap() {
                PinState::Low
            } else {
                PinState::High
            };

            if ready_state == current_state {
                debug!("INPUT CHANNEL: pin in ready state");
                Poll::Ready(())
            } else {
                let task_id = cx.waker().task_id();
                debug!(
                    "INPUT CHANNEL: pin not ready, store pending task with id = {}",
                    task_id
                );
                WAKE_TASKS[self.index].store(task_id, Ordering::Relaxed);
                Poll::Pending
            }
        })
        .await
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    info!("GPIO INTERRUPT: button press detected!");

    // SAFETY: only accessed in IRQ
    let io = unsafe { &*pac::IO_BANK0::ptr() };
    let ints1 = io.proc0_ints(1).read().bits();

    for (i, task) in WAKE_TASKS.iter().enumerate() {
        debug!("GPIO INTERRUPT: i = {}", i);

        let gpio = GPIO_PINS[i].load(Ordering::Relaxed);
        if gpio == INVALID_GPIO {
            debug!("GPIO INTERRUPT: no GPIO mapped for index {}", i);
            continue;
        }

        let low_mask = mask_for(gpio, true);
        let high_mask = mask_for(gpio, false);
        let clear_mask = ints1 & (low_mask | high_mask);

        if clear_mask != 0 {
            debug!("GPIO INTERRUPT: pin {} had edge change!", gpio);
            io.intr(1).write(|w| unsafe { w.bits(clear_mask) });

            let task_id = task.load(Ordering::Relaxed);
            task.store(INVALID_TASK_ID, Ordering::Relaxed);

            if task_id != INVALID_TASK_ID {
                debug!("GPIO INTERRUPT: wake task with ID = {}", task_id);
                wake_task(task_id);
            } else {
                debug!("GPIO INTERRUPT: invalid task ID, no wake task called!")
            }
        } else {
            debug!("GPIO INTERRUPT: pin {} no edge, nothing to do...", gpio);
        }
    }
}

#[inline]
fn bit_offset(edge_low: bool) -> usize {
    if edge_low { 2 } else { 3 }
}

#[inline]
fn mask_for(gpio: usize, edge_low: bool) -> u32 {
    let group = gpio % 8;
    let bit = group * 4 + bit_offset(edge_low);
    1u32 << bit
}
