use std::time::Instant;

use crate::print::{TICK_INTERVAL, Tick};

/// Estimate the time for progress bar
pub struct Estimater {
    /// Time when the progress started
    start: Instant,
    /// If the ETA is accurate enough to be displayed
    is_reasonably_accurate: bool,
    /// Step number when we last estimated ETA
    last_step: u64,
    /// Tick number when we last estimated ETA
    last_tick: u32,
    /// Last calculation, in seconds
    previous_eta: f32,
}

impl Estimater {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            is_reasonably_accurate: false,
            last_step: 0,
            last_tick: 0,
            previous_eta: 0.0,
        }
    }

    pub fn update(
        &mut self,
        now: &mut Option<Instant>,
        current: u64,
        total: u64,
        tick: Tick,
    ) -> Option<f32> {
        let now = match now {
            None => {
                let n = Instant::now();
                *now = Some(n);
                n
            }
            Some(n) => *n,
        };
        let elapsed = (now - self.start).as_secs_f32();
        let secs_per_step = elapsed / current as f32;
        let mut eta = secs_per_step * (total - current) as f32;
        if current == self.last_step {
            // subtract time passed since updating to this step
            let elapased_since_current = (TICK_INTERVAL * (tick - self.last_tick)).as_secs_f32();
            if elapased_since_current > eta {
                self.last_step = current;
                self.last_tick = tick;
            }
            eta = (eta - elapased_since_current).max(0.0);
            // only start showing ETA if it's reasonably accurate
            if !self.is_reasonably_accurate && eta < self.previous_eta - TICK_INTERVAL.as_secs_f32()
            {
                self.is_reasonably_accurate = true;
            }
            self.previous_eta = eta;
        } else {
            self.last_step = current;
            self.last_tick = tick;
        }

        if !self.is_reasonably_accurate {
            None
        } else {
            Some(eta)
        }
    }
}
