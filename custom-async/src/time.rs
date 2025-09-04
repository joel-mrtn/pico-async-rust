use rp_pico::{self as bsp, hal::fugit::TimerDurationU32};

use bsp::hal::{
    self,
    clocks::ClocksManager,
    fugit::{TimerDurationU64, TimerInstantU64},
    pac::{self, interrupt},
    timer::Alarm,
};
use core::{cell::RefCell, task::Poll};
use critical_section::Mutex;
use defmt::{debug, info};
use heapless::Vec;

use crate::executor::{ExtWaker, wake_task};

pub type Instant = TimerInstantU64<1_000_000>;
pub type Duration = TimerDurationU64<1_000_000>;

static NEXT_DEADLINES: Mutex<RefCell<Vec<(u64, usize), 8>>> = Mutex::new(RefCell::new(Vec::new()));

enum TimerState {
    Init,
    Wait,
}

pub struct Timer {
    end_time: Instant,
    state: TimerState,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            end_time: Ticker::now() + duration,
            state: TimerState::Init,
        }
    }

    fn register(&self, task_id: usize) {
        let deadline = self.end_time.duration_since_epoch().ticks();

        critical_section::with(|cs| {
            let mut deadlines = NEXT_DEADLINES.borrow_ref_mut(cs);
            if deadlines.push((deadline, task_id)).is_err() {
                panic!("Too many concurrent timers!");
            }

            let ticker = &mut TICKER.borrow_ref_mut(cs);
            let ticker = ticker.as_mut().unwrap();
            let now = ticker.timer.get_counter().ticks();

            let min_deadline = deadlines
                .iter()
                .filter(|&&(dl, _)| dl > now)
                .map(|&(dl, _)| dl)
                .min();

            ticker.alarm0.clear_interrupt();
            if let Some(min_dl) = min_deadline {
                let duration_ticks = min_dl.saturating_sub(now);
                if duration_ticks > u32::MAX as u64 {
                    panic!("Timer duration too large for u32!");
                }
                let duration = TimerDurationU32::from_ticks(duration_ticks as u32);
                ticker.alarm0.schedule(duration).unwrap();
                ticker.alarm0.enable_interrupt();
            }
        });
    }
}

impl Future for Timer {
    type Output = ();
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        match self.state {
            TimerState::Init => {
                self.register(cx.waker().task_id());
                self.state = TimerState::Wait;
                Poll::Pending
            }
            TimerState::Wait => {
                if Ticker::now() >= self.end_time {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub async fn delay(duration: Duration) {
    Timer::new(duration).await;
}

static TICKER: Mutex<RefCell<Option<Ticker>>> = Mutex::new(RefCell::new(None));

pub struct Ticker {
    timer: hal::Timer,
    alarm0: hal::timer::Alarm0,
}

impl Ticker {
    pub fn init(timer: pac::TIMER, resets: &mut pac::RESETS, clocks: &ClocksManager) {
        let mut timer = hal::Timer::new(timer, resets, clocks);
        let alarm0 = timer.alarm_0().unwrap();

        critical_section::with(|cs| {
            *TICKER.borrow_ref_mut(cs) = Some(Ticker { timer, alarm0 });
        });

        unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) }
    }

    pub fn now() -> Instant {
        critical_section::with(|cs| TICKER.borrow_ref(cs).as_ref().unwrap().timer.get_counter())
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    info!("TIMER INTERRUPT: timer deadline reached!");

    critical_section::with(|cs| {
        let ticker = &mut TICKER.borrow_ref_mut(cs);
        let ticker = ticker.as_mut().unwrap();
        let now = ticker.timer.get_counter().ticks();

        let mut deadlines = NEXT_DEADLINES.borrow_ref_mut(cs);
        let mut to_wake: Vec<usize, 8> = Vec::new();
        let mut remaining: Vec<(u64, usize), 8> = Vec::new();

        for &(dl, tid) in deadlines.iter() {
            if dl <= now {
                debug!("TIMER INTERRUPT: wake task with ID = {}", tid);
                to_wake.push(tid).ok();
            } else {
                remaining.push((dl, tid)).ok();
            }
        }

        *deadlines = remaining;

        for tid in to_wake {
            wake_task(tid);
        }

        ticker.alarm0.clear_interrupt();

        // Reschedule next min if any
        let min_deadline = deadlines
            .iter()
            .filter(|&&(dl, _)| dl > now)
            .map(|&(dl, _)| dl)
            .min();
        if let Some(min_dl) = min_deadline {
            let duration_ticks = min_dl.saturating_sub(now);
            if duration_ticks > u32::MAX as u64 {
                panic!("Timer duration too large for u32!");
            }
            let duration = TimerDurationU32::from_ticks(duration_ticks as u32);
            ticker.alarm0.schedule(duration).unwrap();
            ticker.alarm0.enable_interrupt();
        }
    });
}
