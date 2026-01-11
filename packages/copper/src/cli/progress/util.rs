use std::sync::{Arc, Weak};
use std::time::Instant;

use crate::cli::Tick;
use crate::cli::fmt::ansi;
use crate::cli::progress::ProgressBar;

#[derive(Debug)]
pub enum ChildState {
    /// The done message (if `keep` is true)
    Done(String),
    /// The interrupted message
    Interrupted(String),
    /// Still running
    Progress(usize, Weak<ProgressBar>),
}
impl ChildState {
    pub fn upgrade(&self) -> Option<ChildStateStrong<'_>> {
        Some(match self {
            ChildState::Done(x) => ChildStateStrong::Done(x),
            ChildState::Interrupted(x) => ChildStateStrong::Interrupted(x),
            ChildState::Progress(_, weak) => ChildStateStrong::Progress(weak.upgrade()?),
        })
    }
}

pub enum ChildStateStrong<'a> {
    Done(&'a str),
    Interrupted(&'a str),
    Progress(Arc<ProgressBar>),
}

#[derive(Default, Clone)]
pub enum BarResult {
    /// Bar is done and don't keep it
    #[default]
    DontKeep,
    /// Bar is done, with a message to keep
    Done(String),
    /// Bar is interrupted
    Interrupted(String),
}

pub struct BarFormatter<'a, 'b, 'c> {
    pub colors: ansi::Colors,
    pub bar_color: &'a str,
    pub width: usize,
    pub tick: Tick,
    pub now: &'c mut Option<Instant>,
    pub out: &'b mut String,
    pub temp: &'b mut String,
}
