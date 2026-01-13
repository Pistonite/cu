use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::cli::progress::{Estimater, ProgressBar, State, StateImmut};

/// Builder for a progress bar
#[derive(Debug, Clone)] // Clone sometimes needed to build by ref.. without unsafe
pub struct ProgressBarBuilder {
    /// The message prefix for the progress bar
    message: String,
    /// Total steps (None = unbounded, 0 = not known yet)
    total: Option<u64>,
    /// The progress bar is for displaying bytes
    total_is_in_bytes: bool,
    /// If the bar should be kept after it's done
    keep: Option<bool>,
    /// If ETA should be visible (only effective if total is finite)
    show_eta: bool,
    /// If percentage should be visible (only effective if total is finite)
    show_percentage: bool,
    /// Message to display after done, instead of the default
    done_message: Option<String>,
    /// Message to display if the bar is interrupted
    interrupted_message: Option<String>,
    /// Maximum number of children to display at a time
    max_display_children: usize,
    /// Optional parent of the bar
    parent: Option<Arc<ProgressBar>>,
}

impl ProgressBarBuilder {
    /// Start building a progress bar. Note [`cu::progress`](fn@crate::progress) is the canonical shorthand
    pub fn new(message: String) -> Self {
        Self {
            message,
            total: None,
            total_is_in_bytes: false,
            keep: None,
            show_eta: true,
            show_percentage: true,
            done_message: None,
            interrupted_message: None,
            max_display_children: usize::MAX / 2,
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
        self.total = Some(total);
        self.total_is_in_bytes = true;
        self
    }

    /// Set if the progress bar should be kept in the output
    /// after it's done.
    ///
    /// Default is `true` for root progress bars and `false`
    /// for child progress bars
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").keep(false);
    /// ```
    #[inline(always)]
    pub fn keep(mut self, keep: bool) -> Self {
        self.keep = Some(keep);
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

    /// Set the max number of children to display at a time.
    /// Default is unbounded.
    ///
    /// ```rust
    /// # use pistonite_cu as cu;
    /// cu::progress("doing something").max_display_children(30);
    /// ```
    pub fn max_display_children(mut self, num: usize) -> Self {
        self.max_display_children = num;
        self
    }

    /// Set the parent progress bar.
    ///
    /// If the parent is known to be `Some`, use `parent.child(...)` instead
    pub fn parent(mut self, parent: Option<Arc<ProgressBar>>) -> Self {
        self.parent = parent;
        self
    }

    /// Build and start displaying the bar in the console
    pub fn spawn(self) -> Arc<ProgressBar> {
        let keep = self.keep.unwrap_or(self.parent.is_none());
        let done_message = if keep {
            match self.done_message {
                None => {
                    if self.message.is_empty() {
                        Some("done".to_string())
                    } else {
                        Some(format!("{}: done", self.message))
                    }
                }
                Some(x) => Some(x),
            }
        } else {
            None
        };
        let state_immut = StateImmut {
            id: crate::next_atomic_usize(),
            parent: self.parent.as_ref().map(Arc::clone),
            prefix: self.message,
            done_message,
            interrupted_message: self.interrupted_message,
            show_percentage: self.show_percentage,
            unbounded: self.total.is_none(),
            display_bytes: self.total_is_in_bytes,
            max_display_children: self.max_display_children,
        };
        let eta = self.show_eta.then(Estimater::new);
        let state = State::new(self.total.unwrap_or(0), eta);

        ProgressBar::spawn(state_immut, state, self.parent)
    }
}

