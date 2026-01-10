mod init;

pub use init::*;
pub(crate) mod ansi;
mod printer;
pub(crate) mod utf8;
pub use printer::*;
mod format;
pub use format::*;
mod progress;
pub use progress::*;
mod zero_when_drop;
pub use zero_when_drop::*;

mod prompt;
pub use prompt::*;
#[cfg(feature = "prompt-password")]
mod prompt_password;
#[cfg(feature = "prompt-password")]
pub use prompt_password::check_password_legality;

// 50ms between each cycle
pub(crate) const TICK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
// 2B ticks * 10ms = 251 days.
// overflown tick means ETA will be inaccurate (after 251 days)
pub(crate) type Tick = u32;
