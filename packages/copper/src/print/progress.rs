use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::ansi;

/// Update a progress bar
///
/// # Examples
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let bar = cu::progress_bar(10, "10 steps");
/// // update the current count and message
/// let i = 1;
/// cu::progress!(&bar, i, "doing step {i}");
/// // update the current count without changing message
/// cu::progress!(&bar, 2);
/// // update the message without changing count (or the bar is unbounded)
/// cu::progress!(&bar, (), "doing the thing");
/// ```
#[macro_export]
macro_rules! progress {
    ($bar:expr, $current:expr) => {
        $crate::ProgressBar::set($bar, $current, None);
    };
    ($bar:expr, (), $($fmt_args:tt)*) => {{
        let message = format!($($fmt_args)*);
        $crate::ProgressBar::set_message($bar, message);
    }};
    ($bar:expr, $current:expr, $($fmt_args:tt)*) => {{
        let message = format!($($fmt_args)*);
        $crate::ProgressBar::set($bar, $current, Some(message));
    }};
}

/// Create a progress bar
pub fn progress_bar(total: usize, message: impl Into<String>) -> Arc<ProgressBar> {
    let bar = Arc::new(ProgressBar::new(true, total, message.into()));
    if let Ok(mut printer) = super::PRINTER.lock() {
        printer.add_progress_bar(&bar);
    }
    bar
}

/// Create a progress bar that doesn't print a done message
pub fn progress_bar_lowp(total: usize, message: impl Into<String>) -> Arc<ProgressBar> {
    let bar = Arc::new(ProgressBar::new(false, total, message.into()));
    if let Ok(mut printer) = super::PRINTER.lock() {
        printer.add_progress_bar(&bar);
    }
    bar
}

/// Create a progress bar that doesn't display the current/total
///
/// This is equipvalent to calling `progress_bar` with a total of 0
pub fn progress_unbounded(message: impl Into<String>) -> Arc<ProgressBar> {
    progress_bar(0, message)
}

/// Create a progress bar that doesn't display the current/total, and disappears
/// after done
///
/// This is equipvalent to calling `progress_bar` with a total of 0
pub fn progress_unbounded_lowp(message: impl Into<String>) -> Arc<ProgressBar> {
    progress_bar_lowp(0, message)
}

/// Handle for a progress bar.
///
/// The [`progress`](crate::progress) macro is used to update
/// the bar using a handle
pub struct ProgressBar {
    print_done: bool,
    inner: Mutex<ProgressBarState>,
}
impl Drop for ProgressBar {
    fn drop(&mut self) {
        let (total, message) = {
            match self.inner.lock() {
                Ok(mut bar) => (bar.total, std::mem::take(&mut bar.prefix)),
                Err(_) => (0, String::new()),
            }
        };
        let handle = if let Ok(mut x) = super::PRINTER.lock() {
            if self.print_done {
                x.print_bar_done(&format_bar_done(total, &message));
            }
            x.take_print_task_if_should_join()
        } else {
            None
        };
        if let Some(x) = handle {
            let _: Result<(), _> = x.join();
        }
    }
}
impl ProgressBar {
    fn new(print_done: bool, total: usize, prefix: String) -> Self {
        Self {
            print_done,
            inner: Mutex::new(ProgressBarState::new(total, prefix)),
        }
    }
    /// Set the counter and message of the progress bar.
    ///
    /// Typically, this is done throught the [`cu::progress`](crate::progress)
    /// macro instead of calling this directly
    pub fn set(self: &Arc<Self>, current: usize, message: Option<String>) {
        if let Ok(mut bar) = self.inner.lock() {
            bar.current = current;
            if let Some(x) = message {
                bar.message = x;
            }
        }
    }
    /// Set the message of the progress bar, without changing counter.
    ///
    /// Typically, this is done throught the [`cu::progress`](crate::progress)
    /// macro instead of calling this directly
    pub fn set_message(self: &Arc<Self>, message: String) {
        if let Ok(mut bar) = self.inner.lock() {
            bar.message = message;
        }
    }
    /// Set the total counter. This can be used in cases where the total
    /// count isn't known from the beginning.
    pub fn set_total(self: &Arc<Self>, total: usize) {
        if let Ok(mut bar) = self.inner.lock() {
            bar.set_total(total);
        }
    }
    pub(crate) fn format(
        &self,
        width: usize,
        now: Instant,
        tick: u32,
        tick_interval: Duration,
        out: &mut String,
        temp: &mut String,
    ) {
        if let Ok(mut bar) = self.inner.lock() {
            bar.format(width, now, tick, tick_interval, out, temp)
        }
    }
}

/// Progress bar state
struct ProgressBarState {
    /// Total count, or 0 for unbounded
    total: usize,
    /// Current count, has no meaning for unbounded
    current: usize,
    /// Current when we last estimated ETA
    last_eta_current: usize,
    /// Tick when we last estimated ETA
    last_eta_tick: u32,
    /// Last calculation
    previous_eta: f64,
    /// If ETA should be shown, we only show if it's reasonably accurate
    should_show_eta: bool,
    /// Prefix to display, usually indicating what the progress bar is for
    prefix: String,
    /// Message to display, usually indicating what the current action is
    message: String,
    /// If bounded, used for estimating the ETA
    started: Instant,
}

impl ProgressBarState {
    pub(crate) fn new(total: usize, prefix: String) -> Self {
        Self {
            total,
            current: 0,
            last_eta_current: 0,
            last_eta_tick: 0,
            previous_eta: 0f64,
            should_show_eta: false,
            started: Instant::now(),
            prefix,
            message: String::new(),
        }
    }
    pub(crate) fn set_total(&mut self, total: usize) {
        self.total = total;
        self.current = self.current.min(total);
    }
    pub(crate) fn is_unbounded(&self) -> bool {
        self.total == 0
    }
    /// Format the progress bar, adding at most `width` bytes to the buffer,
    /// not including a newline
    pub(crate) fn format(
        &mut self,
        mut width: usize,
        now: Instant,
        tick: u32,
        tick_interval: Duration,
        out: &mut String,
        temp: &mut String,
    ) {
        use std::fmt::Write;
        // format: [current/total] prefix: DD.DD% ETA SS.SSs message
        match width {
            0 => return,
            1 => {
                out.push('.');
                return;
            }
            2 => {
                out.push_str("..");
                return;
            }
            3 => {
                out.push_str("...");
                return;
            }
            4 => {
                out.push_str("[..]");
                return;
            }
            _ => {}
        }
        temp.clear();
        if !self.is_unbounded() {
            if write!(temp, "{}/{}", self.current, self.total).is_err() {
                temp.clear();
            }
            // .len() is safe because / and numbers have the same byte size and width
            // -2 is safe because width > 4 here
            if temp.len() > width - 2 {
                out.push('[');
                for _ in 0..(width - 2) {
                    out.push('.');
                }
                out.push(']');
                return;
            }

            width -= 2;
            width -= temp.len();
            out.push('[');
            out.push_str(temp);
            out.push(']');
        }
        if width > 0 {
            out.push(' ');
            width -= 1;
        }
        for (c, w) in ansi::with_width(self.prefix.chars()) {
            if w > width {
                break;
            }
            width -= w;
            out.push(c);
        }
        if !self.is_unbounded() && self.current > 0 {
            let start = self.started;
            let elapsed = (now - start).as_secs_f64();
            // show percentage/ETA if the progress takes more than 2s
            if elapsed > 2f64 && self.current <= self.total {
                // percentage
                // : DD.DD% or : 100%
                if self.current == self.total {
                    if self.prefix.is_empty() {
                        if width >= 4 {
                            width -= 4;
                            out.push_str("100%");
                        }
                    } else {
                        if width >= 6 {
                            width -= 6;
                            out.push_str(": 100%");
                        }
                    }
                } else {
                    let percentage = self.current as f32 * 100f32 / self.total as f32;
                    temp.clear();
                    if self.prefix.is_empty() {
                        if write!(temp, "{percentage:.2}%").is_err() {
                            temp.clear();
                        }
                    } else {
                        if write!(temp, ": {percentage:.2}%").is_err() {
                            temp.clear();
                        }
                    }
                    if width >= temp.len() {
                        width -= temp.len();
                        out.push_str(temp);
                    }
                }
                // ETA SS.SSs
                let secs_per_unit = elapsed / self.current as f64;
                let mut eta = secs_per_unit * (self.total - self.current) as f64;
                if self.current == self.last_eta_current {
                    // subtract time passed since updating to this step
                    let elapased_since_current =
                        (tick_interval * (tick - self.last_eta_tick)).as_secs_f64();
                    if elapased_since_current > eta {
                        self.last_eta_current = self.current;
                        self.last_eta_tick = tick;
                    }
                    eta = (eta - elapased_since_current).max(0f64);
                    // only start showing ETA if it's reasonably accurate
                    if !self.should_show_eta
                        && eta < self.previous_eta - tick_interval.as_secs_f64()
                    {
                        self.should_show_eta = true;
                    }
                    self.previous_eta = eta;
                } else {
                    self.last_eta_current = self.current;
                    self.last_eta_tick = tick;
                }
                if self.should_show_eta {
                    if width > 0 {
                        out.push(' ');
                        width -= 1;
                    }
                    temp.clear();
                    if write!(temp, "ETA {eta:.2}s;").is_err() {
                        temp.clear();
                    }
                    if width >= temp.len() {
                        width -= temp.len();
                        out.push_str(temp);
                    }
                }
            } else {
                if !self.prefix.is_empty() && !self.message.is_empty() && width > 0 {
                    out.push(':');
                    width -= 1;
                }
            }
            if width > 0 {
                out.push(' ');
                width -= 1;
            }
        } else {
            if !self.prefix.is_empty() && !self.message.is_empty() && width > 1 {
                out.push_str(": ");
                width -= 2;
            }
        }
        for (c, w) in ansi::with_width(self.message.chars()) {
            if w > width {
                break;
            }
            width -= w;
            out.push(c);
        }
    }
}

fn format_bar_done(total: usize, message: &str) -> String {
    if total == 0 {
        if message.is_empty() {
            "\u{283f}] done".to_string()
        } else {
            format!("\u{283f}] {message}: done")
        }
    } else {
        if message.is_empty() {
            format!("\u{283f}][{total}/{total}] done")
        } else {
            format!("\u{283f}][{total}/{total}] {message}: done")
        }
    }
}
