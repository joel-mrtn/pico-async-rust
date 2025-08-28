use rp_pico as bsp;

use bsp::hal::{
    self,
    clocks::ClocksManager,
    fugit::{TimerDurationU64, TimerInstantU64},
    pac,
};

pub type Instant = TimerInstantU64<1_000_000>;
pub type Duration = TimerDurationU64<1_000_000>;

pub struct Timer<'a> {
    end_time: Instant,
    ticker: &'a Ticker,
}

impl<'a> Timer<'a> {
    pub fn new(duration: Duration, ticker: &'a Ticker) -> Self {
        Self {
            end_time: ticker.now() + duration,
            ticker,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ticker.now() >= self.end_time
    }
}

pub struct Ticker {
    timer: hal::Timer,
}

impl Ticker {
    pub fn new(timer: pac::TIMER, resets: &mut pac::RESETS, clocks: &ClocksManager) -> Self {
        let timer = hal::Timer::new(timer, resets, clocks);

        Self { timer }
    }

    pub fn now(&self) -> Instant {
        Instant::from_ticks(self.timer.get_counter().ticks())
    }
}
