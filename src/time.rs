use rp_pico as bsp;

use bsp::hal::{
    self,
    clocks::ClocksManager,
    fugit::{TimerDurationU64, TimerInstantU64},
    pac::{self, interrupt},
    timer::Alarm,
};
use core::{cell::RefCell, task::Poll};
use critical_section::Mutex;

use crate::executor::{ExtWaker, wake_task};

pub type Instant = TimerInstantU64<1_000_000>;
pub type Duration = TimerDurationU64<1_000_000>;

static NEXT_DEADLINE: Mutex<RefCell<Option<(u64, usize)>>> = Mutex::new(RefCell::new(None));

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
            *NEXT_DEADLINE.borrow_ref_mut(cs) = Some((deadline, task_id));
            let ticker = &mut TICKER.borrow_ref_mut(cs);
            let ticker = ticker.as_mut().unwrap();

            ticker.alarm0.schedule_at(self.end_time).unwrap();
            ticker.alarm0.enable_interrupt();
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

        unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };
    }

    pub fn now() -> Instant {
        critical_section::with(|cs| TICKER.borrow_ref(cs).as_ref().unwrap().timer.get_counter())
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    critical_section::with(|cs| {
        if let Some((_deadline, task_id)) = NEXT_DEADLINE.borrow_ref_mut(cs).take() {
            wake_task(task_id);
        }

        let ticker = &mut TICKER.borrow_ref_mut(cs);
        let ticker = ticker.as_mut().unwrap();

        ticker.alarm0.clear_interrupt();
    });
}
