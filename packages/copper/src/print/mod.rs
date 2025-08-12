#[cfg(feature = "print")]
mod init;

#[cfg(feature = "print")]
pub use init::*;
#[cfg(feature = "print")]
pub(crate) mod ansi;
mod level;
pub(crate) mod utf8;
pub use level::*;
#[cfg(feature = "print")]
mod printer;
#[cfg(feature = "print")]
pub(crate) use printer::*;
#[cfg(feature = "print")]
mod format;
#[cfg(feature = "print")]
pub use format::*;
#[cfg(feature = "print")]
mod progress;
#[cfg(feature = "print")]
pub use progress::*;

#[cfg(feature = "print")]
mod prompt;
#[cfg(feature = "print")]
pub use prompt::*;

use std::sync::{atomic::{AtomicBool, Ordering}, OnceLock};
use crate::Atomic;

pub(crate) static PRINT_LEVEL: Atomic<u8, PrintLevel> = Atomic::new_u8(PrintLevel::Normal as u8);
static USE_COLOR: AtomicBool = AtomicBool::new(true);
#[allow(unused, clippy::type_complexity)]
pub(crate) static PRINT_DELEGATE: OnceLock<Box<dyn Fn(Lv, std::fmt::Arguments<'_>) + Send + Sync + 'static>> = OnceLock::new();

/// Check if the logging level is enabled
pub fn log_enabled(lv: Lv) -> bool {
    lv.can_print(PRINT_LEVEL.get())
}

/// Get if color printing is enabled
pub fn color_enabled() -> bool {
    USE_COLOR.load(Ordering::Acquire)
}

pub fn __print_with_level(lv: Lv, message: std::fmt::Arguments<'_>) {
    let Some(delegate) = PRINT_DELEGATE.get() else {
        return;
    };
    (delegate)(lv, message)
}
