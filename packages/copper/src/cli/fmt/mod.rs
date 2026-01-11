pub mod ansi;
pub mod utf8;

mod term_size;
pub use term_size::*;
mod format_buffer;
pub(crate) use format_buffer::*;
