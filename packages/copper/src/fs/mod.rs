//! # File System Operations
//!
//! Much of this is WIP and I will fill up this documentation in the future.
//! The general principle of this is `cu::fs` functions have tracing and error
//! context built-in

mod dir;
pub use dir::*;
mod file;
pub use file::*;
mod read;
pub use read::*;
mod write;
pub use write::*;
mod walk;
pub use walk::*;
mod glob;
pub use glob::*;
pub mod bin;

pub use filetime::FileTime as Time;
