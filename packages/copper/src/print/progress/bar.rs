use std::{ops::Deref, sync::{Arc, Mutex}};

use crate::print::progress::state::{State, StateImmut};

/// Make a progress bar builder with the following defaults:
///
/// - Total steps: unbounded
/// - Keep after done: `true`
/// - Show ETA: `true` (only effective if steps is finite)
/// - Finish message: Default
/// - Interrupted message: Default
///
/// See [`ProgressBarBuilder`] for builder methods
#[inline(always)]
pub fn progress(message: impl Into<String>) -> ProgressBarBuilder {
    ProgressBarBuilder::new(message.into())
}

/// Builder for a progress bar
pub struct ProgressBarBuilder {
    /// The message prefix for the progress bar
    message: String,
    /// Total steps (None = unbounded, 0 = not known yet)
    total: Option<u64>,
    /// The progress bar is for displaying bytes
    total_is_in_bytes: bool,
    /// If the bar should be kept after it's done
    keep: bool,
    /// If ETA should be visible (only effective if total is finite)
    show_eta: bool,
    /// If percentage should be visible (only effective if total is finite)
    show_percentage: bool,
    /// Message to display after done, instead of the default
    done_message: Option<String>,
    /// Message to display if the bar is interrupted
    interrupted_message: Option<String>,
    /// Optional parent of the bar
    parent: Option<Arc<ProgressBar>>,
}
impl ProgressBarBuilder {
    /// Start building a progress bar. Note [`cu::progress`](progress) is the canonical shorthand
    pub fn new(message: String) -> Self {
        Self { 
            message,
            total: None,
            total_is_in_bytes: false,
            keep: true,
            show_eta: true,
            show_percentage: true,
            done_message: None,
            interrupted_message: None,
            parent: None,
        }
    }
    /// Set the total steps. `0` means total is unknown, which can be set
    /// at a later point.
    ///
    /// By default, the progress bar is "unbounded", meaning there is no
    /// individual steps
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").total(10);
    /// ```
    #[inline(always)]
    pub fn total(mut self, total: usize) -> Self {
        self.total = Some(total as u64);
        self
    }

    /// Set the total as a `u64` on platforms where `usize` is less than 64 bits
    #[cfg(not(target_pointer_width = "64"))]
    pub fn total_u64(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// Set the total bytes and set the progress to be displayed using byte units (SI).
    /// `0` means total is unknown, which can be set at a later point.
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").total_bytes(1000000);
    /// ```
    #[inline(always)]
    pub fn total_bytes(mut self, total: u64) -> Self {
        self.total = Some(total as u64);
        self.total_is_in_bytes = true;
        self
    }

    /// Set if the progress bar should be kept in the output
    /// after it's done. Default is `true`
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").keep(false);
    /// ```
    #[inline(always)]
    pub fn keep(mut self, keep: bool) -> Self {
        self.keep = keep;
        self
    }

    /// Set if ETA (estimated time) should be displayed.
    /// Only effective if total is not zero (i.e. not unbounded).
    /// Default is `true`
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").total(10).eta(false);
    /// ```
    #[inline(always)]
    pub fn eta(mut self, show: bool) -> Self {
        self.show_eta = show;
        self
    }

    /// Set if percentage should be displayed.
    /// Only effective if total is not zero (i.e. not unbounded).
    /// Default is `true`
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").total(10).percentage(false);
    /// ```
    #[inline(always)]
    pub fn percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Set a message to be displayed when the progress is done.
    /// Requires `keep(true)` - which is the default, but
    /// `when_done` will not automatically turn it on for you.
    ///
    /// Default is the message of the progress bar followed by `done`.
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").when_done("something is done!");
    /// ```
    #[inline(always)]
    pub fn when_done(mut self, message: impl Into<String>) -> Self {
        self.done_message = Some(message.into());
        self
    }

    /// Set a message to be displayed when the progress is interrupted.
    ///
    /// Default is the message of the progress bar followed by `interrupted`.
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").when_interrupt("something is interrupted!");
    /// ```
    #[inline(always)]
    pub fn when_interrupt(mut self, message: impl Into<String>) -> Self {
        self.interrupted_message = Some(message.into());
        self
    }

    /// Build and start displaying the bar in the console
    pub fn spawn(self) -> Arc<ProgressBar> {
        todo!()
    }
}

trait IntoProgress {
    fn into_progress(self) -> u64;
}
#[rustfmt::skip]
const _: () = {
    impl IntoProgress for u64 { #[inline(always)] fn into_progress(self) -> u64 { self } }
    impl IntoProgress for u32 { #[inline(always)] fn into_progress(self) -> u64 { self } }
    impl IntoProgress for u16 { #[inline(always)] fn into_progress(self) -> u64 { self } }
    impl IntoProgress for u8 { #[inline(always)] fn into_progress(self) -> u64 { self } }
    impl IntoProgress for usize { #[inline(always)] fn into_progress(self) -> u64 { self as u64 } }
};

pub struct ProgressBar {
    pub(crate) props: StateImmut,
    state: Mutex<State>,
}
impl ProgressBar {
    #[doc(hidden)]
    #[inline(always)]
    pub fn __set<T: IntoProgress>(self: &Arc<Self>, current: T, message: Option<String>) {
        Self::set(self, current.into_progress(), message)
    }
    fn set(self: &Arc<Self>, current: u64, message: Option<String>) {
        if let Ok(mut bar) = self.state.lock() {
            bar.set_current(current);
            if let Some(x) = message {
                bar.set_message(&x);
            }
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn __inc<T: IntoProgress>(self: &Arc<Self>, amount: T, message: Option<String>) {
        Self::inc(self, amount.into_progress(), message)
    }
    fn inc(self: &Arc<Self>, amount: u64, message: Option<String>) {
        if let Ok(mut bar) = self.state.lock() {
            bar.inc_current(amount);
            if let Some(x) = message {
                bar.set_message(&x);
            }
        }
    }

    /// Set the total steps (if the progress is finite)
    #[inline(always)]
    pub fn set_total<T: IntoProgress>(&self, total: T) {
        Self::set_total_impl(self, total.into_progress())
    }
    fn set_total_impl(&self, total: u64) {
        if let Ok(mut bar) = self.state.lock() {
            bar.set_total(total);
        }
    }

    /// Start building a child progress bar
    ///
    /// Note that the child builder will keep this bar alive (displayed), even
    /// if the child is not spawned
    #[inline(always)]
    pub fn child(self: &Arc<Self>, message: impl Into<String>) -> ProgressBarBuilder {
        let mut builder = ProgressBarBuilder::new(message.into());
        builder.parent = Some(Arc::clone(self));
        builder
    }

    /// Mark the progress bar as done and drop the handle.
    ///
    /// This needs to be called if the bar is unbounded. Otherwise,
    /// the bar will display in the interrupted state when dropped.
    ///
    /// If the progress is finite, then interrupted state is automatically
    /// determined (`current != total`)
    pub fn done(self: Arc<Self>) {
        if self.props.unbounded {
            if let Ok(mut bar) = self.state.lock() {
                bar.set_current(1);
                bar.set_total(1);
            }
        }
    }
}
