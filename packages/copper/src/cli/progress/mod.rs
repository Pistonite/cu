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

mod eta;
pub use eta::Estimater;
mod state;
pub use state::ProgressBar;
use state::{State, StateImmut};
mod builder;
pub use builder::ProgressBarBuilder;
mod util;
pub use util::{BarFormatter, BarResult};
use util::{ChildState, ChildStateStrong};
mod macros;

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
