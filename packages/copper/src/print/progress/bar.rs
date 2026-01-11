use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::print::progress::eta::Estimater;
use crate::print::progress::state::{BarFormatter, BarResult, State, StateImmut};

/// # Progress Bars
/// Progress bars are a feature in the print system. It is aware of the printing/prompting going on
/// in the console and will keep the bars at the bottom of the console without interferring
/// with the other outputs.
///
/// ## Components
/// A bar has the following display components
/// - Step display: Displays the current and total steps. For example, `[42/100]`. Will not display
///   for bars that are unbounded. Bars that are not unbounded but the total is not set
///   will show total as `?`. The step display can also be configured to a style more suitable
///   for displaying bytes (for example downloading or processing file), like `10.0K / 97.3M`
/// - Prefix: A string configured once when launching the progress bar
/// - Percentage: Percentage display for the current and total steps, For example `42.00%`.
///   This can be turned off if not needed
/// - ETA: Estimated remaining time. This can be turned off if not needed
/// - Message: A message that can be set while the progress bar is showing. For example,
///   this can be the name of the current file being processed, etc.
///
/// With everything displayed, it will look something like this:
/// ```text
/// X][42/100] prefix: 42.00% ETA 32.35s processing the 42th item
/// ```
/// (`X`) is where the animated spinner is
///
/// ## Progress Tree
/// You can display progress bars with a hierarchy if desired. The progress bars
/// will be organized as an directed acyclic graph (i.e. a tree). Special characters
/// will be used to draw the tree in the terminal.
///
/// Each progress bar holds a strong ref to its parent, and weak refs to all of its children.
/// The printer keeps weak refs to all root progress bars (i.e. one without a parent).
///
/// ## State and Output
/// Each progress bar can have 3 states: `progress`, `done`, and `interrupted`.
///
/// When in `progress`, the bar will be animated if the output is a terminal. Otherwise,
/// updates will be ignored.
///
/// The bar will be `done` when all handles are dropped if 1 of the following is true:
/// - The bar has finite total, and current step equals total step
/// - The bar is unbounded, and `.done()` is called on any handle
///
/// If neither is true when all handles are dropped, the bar becomes `interrupted`.
/// This makes the bar easier to use with control flows. When the bar is in this state,
/// it will print an interrupted message to the regular print stream, like
/// ```text
/// X][42/100] prefix: interrupted
/// ```
/// This message is customizable when building the progress bar. All of its children
/// that are interrupted will also be printed. All children that are `done` will only be
/// printed if `keep` is true for that children (see below). The interrupted message is printed
/// regardless if the output is terminal or not.
///
/// When the progress bar is done, it may print a "done message" depending on
/// if it has a parent and the `keep` option:
/// | Has parent (i.e. is child) | Keep | Behavior |
/// |-|-|-|
/// | Yes | Yes | Done message will be displayed under the parent, but the bar will disappear completely when the parent is done |
/// | Yes | No  | The bar will disappear after it's done |
/// | No  | Yes | The bar will print a done message to the regular print stream when done, no children will be printed |
/// | No  | No  | The bar will disappear after done, no children will be printed |
///
/// The done message is also customizable when building the bar. Note (from the table) that it will
/// be effective in some way if the `keep` option is true. Setting a done message
/// does not automatically set `keep` to true.
///
/// The default done message is something like below, will be displayed in green.
/// ```text
/// X][100/100] prefix: done
/// ```
///
/// ## Updating the bar
/// The [`progress`](macro@crate::progress) macro is used to update the progress bar.
/// For example:
///
/// ```rust
/// # use pistonite_cu as cu;
/// let bar = cu::progress("doing something").total(10).spawn();
/// for i in 0..10 {
///     cu::progress!(bar = i, "doing {i}th step");
/// }
/// drop(bar);
/// ```
///
/// ## Building the bar
/// This function `cu::progress` will make a [`ProgressBarBuilder`]
/// with these default configs:
/// - Total steps: unbounded
/// - Keep after done: `true`
/// - Show ETA: `true` (only effective if steps is finite)
/// - Finish message: Default
/// - Interrupted message: Default
///
/// See [`ProgressBarBuilder`] for builder methods
///
/// ## Print Levels
/// The bar final messages are suppressed at `-q` and the bar animations are suppressed at `-qq`
///
/// ## Other considerations
/// If the progress bar print section exceeds the terminal height,
/// it will probably not render properly. Keep in mind when you
/// are displaying a large number of progress bars.
///
/// You can use `.max_display_children()` to set the maximum number of children
/// to display at a time. However, there is no limit on the number of root progress bars.
#[inline(always)]
pub fn progress(message: impl Into<String>) -> ProgressBarBuilder {
    ProgressBarBuilder::new(message.into())
}

/// Update a [progress bar](fn@crate::progress)
///
/// The macro takes 2 parts separated by comma `,`:
/// - An expression for updating the progress:
/// - Optional format args for updating the message.
///
/// The progress update expression can be one of:
/// - `bar = i`: set the progress to `i`
/// - `bar += i`: increment the steps by i
/// - `bar`: don't update the progress
///
/// , where `bar` is an ident
///
/// The format args can be omitted to update the progress without
/// updating the message
///
/// # Examples
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let bar = cu::progress_bar(10, "10 steps");
/// // update the current count and message
/// let i = 1;
/// cu::progress!(bar = i, "doing step {i}");
/// // update the current count without changing message
/// cu::progress!(bar += 2);
/// // update the message without changing current step
/// cu::progress!(bar, "doing the thing");
/// ```
#[macro_export]
macro_rules! progress {
    ($bar:ident, $($fmt_args:tt)*) => {
        $bar.__inc(0u64, Some(format!($($fmt_args)*)))
    };
    ($bar:ident += $inc:expr) => {
        $bar.__inc({ $inc } as u64, None)
    };
    ($bar:ident += $inc:expr, $($fmt_args:tt)*) => {
        $bar.__inc({ $inc } as u64, Some(format!($($fmt_args)*)))
    };
    ($bar:ident = $x:expr) => {
        $bar.__set({ $x } as u64, None)
    };
    ($bar:ident = $x:expr, $($fmt_args:tt)*) => {
        $bar.__set({ $x } as u64, Some(format!($($fmt_args)*)))
    };
}

// spawn_iter stuff, keep for reference, not sure if needed yet
// .enumerate seems more readable
/*
/// In the example above, you can also attach it to an iterator directly.
/// The builder will call `size_hint()` once and set the total on the bar,
/// and will automatically mark it as done if `next()` returns `None`.
///
/// If the default iteration behavior of `spawn_iter` is not desirable, use `spawn`
/// and iterate manually.
/// ```rust
/// # use pistonite_cu as cu;
/// for i in cu::progress("doing something").spawn_iter(0..10) {
///     cu::print!("doing {i}th step");
/// }
/// ```
///
/// Note that in the code above, we didn't have a handle to the bar directly
/// to update the message, we can fix that by getting the bar from the iter
///
/// ```rust
/// # use pistonite_cu as cu;
/// let mut iter = cu::progress("doing something").spawn_iter(0..10);
/// let bar = iter.bar();
/// for i in iter {
///     // bar = i is handled by the iterator automatically
///     cu::progress!(bar, "doing {i}th step");
/// }
/// ```
*/

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
    /// Start building a progress bar. Note [`cu::progress`](progress) is the canonical shorthand
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
        self.total = Some(total as u64);
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
            id: next_id(),
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

        let bar = Arc::new(ProgressBar {
            state: state_immut,
            state_mut: Mutex::new(state),
        });
        match self.parent {
            Some(p) => {
                if let Ok(mut p) = p.state_mut.lock() {
                    p.add_child(&bar);
                }
            }
            None => {
                if let Ok(mut printer) = super::super::PRINTER.lock() {
                    printer.add_progress_bar(&bar);
                }
            }
        }

        bar
    }
}

fn next_id() -> usize {
    static ID: AtomicUsize = AtomicUsize::new(1);
    ID.fetch_add(1, Ordering::SeqCst)
}
#[derive(Debug)]
pub struct ProgressBar {
    pub(crate) state: StateImmut,
    state_mut: Mutex<State>,
}
impl ProgressBar {
    #[doc(hidden)]
    #[inline(always)]
    pub fn __set(self: &Arc<Self>, current: u64, message: Option<String>) {
        if let Ok(mut bar) = self.state_mut.lock() {
            bar.set_current(current);
            if let Some(x) = message {
                bar.set_message(&x);
            }
        }
    }

    #[doc(hidden)]
    #[inline(always)]
    pub fn __inc(self: &Arc<Self>, amount: u64, message: Option<String>) {
        if let Ok(mut bar) = self.state_mut.lock() {
            bar.inc_current(amount);
            if let Some(x) = message {
                bar.set_message(&x);
            }
        }
    }

    /// Set the total steps (if the progress is finite)
    pub fn set_total(&self, total: u64) {
        if let Ok(mut bar) = self.state_mut.lock() {
            bar.set_total(total);
        }
    }

    /// Start building a child progress bar
    ///
    /// Note that the child builder will keep this bar alive (displayed), even
    /// if the child is not spawned
    #[inline(always)]
    pub fn child(self: &Arc<Self>, message: impl Into<String>) -> ProgressBarBuilder {
        ProgressBarBuilder::new(message.into()).parent(Some(Arc::clone(self)))
    }

    /// Mark the progress bar as done and drop the handle.
    ///
    /// This needs to be called if the bar is unbounded. Otherwise,
    /// the bar will display in the interrupted state when dropped.
    ///
    /// If the progress is finite, then interrupted state is automatically
    /// determined (`current != total`)
    pub fn done(self: Arc<Self>) {
        if self.state.unbounded {
            if let Ok(mut bar) = self.state_mut.lock() {
                bar.set_current(1);
                bar.set_total(1);
            }
        }
    }

    /// Same as [`done`](Self::done), but does not drop the bar.
    pub fn done_by_ref(&self) {
        if self.state.unbounded {
            if let Ok(mut bar) = self.state_mut.lock() {
                bar.set_current(1);
                bar.set_total(1);
            }
        }
    }

    /// Format the bar
    #[inline(always)]
    pub(crate) fn format(&self, fmt: &mut BarFormatter<'_, '_, '_>) -> i32 {
        self.format_at_depth(0, &mut String::new(), fmt)
    }

    /// Format the bar at depth
    pub(crate) fn format_at_depth(
        &self,
        depth: usize,
        hierarchy: &mut String,
        fmt: &mut BarFormatter<'_, '_, '_>,
    ) -> i32 {
        let Ok(mut bar) = self.state_mut.lock() else {
            return 0;
        };
        bar.format_at_depth(depth, hierarchy, fmt, &self.state)
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        let result = match self.state_mut.lock() {
            Err(_) => BarResult::DontKeep,
            Ok(bar) => bar.check_result(&self.state),
        };
        if let Some(parent) = &self.state.parent {
            // inform parent our result
            if let Ok(mut parent_state) = parent.state_mut.lock() {
                parent_state.child_done(self.state.id, result.clone());
            }
        }
        let handle = {
            // scopr for printer lock
            let Ok(mut printer) = super::super::PRINTER.lock() else {
                return;
            };
            printer.print_bar_done(&result, self.state.parent.is_none());
            printer.take_print_task_if_should_join()
        };
        if let Some(x) = handle {
            let _: Result<(), _> = x.join();
        }
    }
}
