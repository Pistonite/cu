
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
