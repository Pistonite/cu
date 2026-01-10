use std::sync::{Arc, Weak};
use std::time::Instant;

use crate::print::progress::bar::ProgressBar;
use crate::print::progress::eta::Estimater;
use crate::print::{Tick, ansi};

const CHAR_BAR_TICK: char = '\u{251C}';
const CHAR_BAR: char = '\u{2502}';
const CHAR_TICK: char = '\u{2514}';

/// Internal, immutable state of progress bar
pub struct StateImmut {
    /// An ID
    pub id: usize,
    /// Parent of this bar
    pub parent: Option<Arc<ProgressBar>>,
    /// The prefix message (corresponds to message in the builder)
    pub prefix: String,
    /// None means don't keep the progress bar printed
    /// (the default done message is formatted at spawn time)
    pub done_message: Option<String>,
    /// None means use the default
    pub interrupted_message: Option<String>,
    /// If percentage field is shown
    pub show_percentage: bool,
    /// If the steps are unbounded
    pub unbounded: bool,
    /// Display the progress using bytes format
    pub display_bytes: bool,
    /// Max number of children to display,
    /// children after the limit will only display one line "... and X more"
    pub max_display_children: usize,
}

/// Internal mutable state
pub struct State {
    unreal_total: u64,
    unreal_current: u64,
    message: String,
    eta: Option<Estimater>,
    children: Vec<ChildState>,
}
impl State {
    pub fn new(total: u64, eta: Option<Estimater>) -> Self {
        Self {
            unreal_total: total,
            unreal_current: 0,
            message: String::new(),
            eta,
            children: vec![],
        }
    }
    #[inline(always)]
    fn estimate_remaining(
        &mut self,
        unbounded: bool,
        now: &mut Option<Instant>,
        tick: Tick,
    ) -> Option<f32> {
        if unbounded || self.unreal_total == 0 {
            return None;
        }
        self.eta.as_mut()?.update(
            now,
            self.unreal_current.min(self.unreal_total),
            self.unreal_total,
            tick,
        )
    }
    #[inline(always)]
    fn real_current_total(&self, unbounded: bool) -> (u64, Option<u64>) {
        if unbounded {
            (0, None)
        } else if self.unreal_total == 0 {
            // total not known
            (self.unreal_current, None)
        } else {
            (
                self.unreal_current.min(self.unreal_total),
                Some(self.unreal_total),
            )
        }
    }

    pub fn set_current(&mut self, current: u64) {
        self.unreal_current = current;
    }

    pub fn inc_current(&mut self, current: u64) {
        self.unreal_current += current;
    }

    pub fn set_total(&mut self, total: u64) {
        if total != 0 {
            self.unreal_total = total;
        }
    }

    pub fn add_child(&mut self, child: &Arc<ProgressBar>) {
        self.children
            .push(ChildState::Progress(child.state.id, Arc::downgrade(child)))
    }

    pub fn child_done(&mut self, child_id: usize, mut result: BarResult) {
        self.children.retain_mut(|child| {
            let ChildState::Progress(id, _) = child else {
                return true;
            };
            if *id != child_id {
                return true;
            }
            match std::mem::take(&mut result) {
                BarResult::DontKeep => false,
                BarResult::Done(message) => {
                    *child = ChildState::Done(message);
                    true
                }
                BarResult::Interrupted(message) => {
                    *child = ChildState::Interrupted(message);
                    true
                }
            }
        });
    }

    pub fn check_result(&self, state: &StateImmut) -> BarResult {
        let is_interrupted = (self.unreal_current == 0 && self.unreal_total == 0)
            || (self.unreal_current < self.unreal_total);
        if !is_interrupted {
            match &state.done_message {
                None => BarResult::DontKeep,
                Some(message) => {
                    let message =
                        self.format_finish_message(message, state.unbounded, state.display_bytes);
                    BarResult::Done(message)
                }
            }
        } else {
            match &state.interrupted_message {
                None => {
                    let message = if state.prefix.is_empty() {
                        self.format_finish_message(
                            "interrupted",
                            state.unbounded,
                            state.display_bytes,
                        )
                    } else {
                        self.format_finish_message(
                            &format!("{}: interrupted", state.prefix),
                            state.unbounded,
                            state.display_bytes,
                        )
                    };
                    BarResult::Interrupted(message)
                }
                Some(message) => {
                    let message =
                        self.format_finish_message(message, state.unbounded, state.display_bytes);
                    BarResult::Interrupted(message)
                }
            }
        }
    }

    pub fn set_message(&mut self, message: &str) {
        self.message.clear();
        self.message.push_str(message);
    }

    /// Format the bar into the out buffer at the depth
    ///
    /// If depth is 0, the animation character is already formatted.
    /// Otherwise, a "| " should be formatted into the out buffer
    /// at the beginning. The `width` passed in is terminal width minus 2.
    ///
    /// It should also format a new line character into the buffer
    ///
    /// Return number of lines formatted.
    pub fn format_at_depth(
        &mut self,
        depth: usize,
        hierarchy: &mut String,
        fmt: &mut BarFormatter<'_, '_, '_>,
        state: &StateImmut,
    ) -> i32 {
        self.format_self(fmt, fmt.width.saturating_sub((depth + 1) * 2), state);
        fmt.out.push('\n');
        let mut lines = 1;
        // process childrens
        let mut i = 0;
        let mut num_displayed = 0;
        let children_count = self.children.len();
        self.children.retain_mut(|child| {
            let out = &mut *fmt.out;
            let Some(child) = child.upgrade() else {
                i += 1;
                return false; // remove the finished child
            };
            if num_displayed >= state.max_display_children {
                num_displayed += 1;
                return true;
            }
            // format the multi-line syntax
            out.push_str(". ");
            out.push_str(fmt.colors.gray);
            out.push_str(hierarchy);
            if i == children_count - 1 {
                out.push(CHAR_TICK);
                hierarchy.push_str("  ");
            } else {
                out.push(CHAR_BAR_TICK);
                hierarchy.push(CHAR_BAR);
                hierarchy.push(' ');
            }
            out.push(' ');
            let width = fmt.width.saturating_sub((depth + 2) * 2);
            match child {
                ChildStateStrong::Done(message) => {
                    out.push_str(fmt.colors.green);
                    format_message_with_width(out, width, message);
                    out.push('\n');
                    lines += 1;
                    out.push_str(fmt.bar_color);
                }
                ChildStateStrong::Interrupted(message) => {
                    out.push_str(fmt.colors.yellow);
                    format_message_with_width(out, width, message);
                    out.push('\n');
                    lines += 1;
                    out.push_str(fmt.bar_color);
                }
                ChildStateStrong::Progress(child) => {
                    out.push_str(fmt.bar_color);
                    lines += child.format_at_depth(depth + 1, hierarchy, fmt);
                }
            }
            hierarchy.pop();
            hierarchy.pop();
            i += 1;
            num_displayed += 1;
            true
        });
        if num_displayed > state.max_display_children {
            // display the ... and more line
            let out = &mut *fmt.out;
            out.push_str("| ");
            out.push_str(fmt.colors.gray);
            for _ in 0..depth {
                out.push(CHAR_BAR);
                out.push(' ');
            }
            out.push(CHAR_TICK);
            use std::fmt::Write as _;
            let _ = write!(
                out,
                "  ... and {} more",
                state.max_display_children - num_displayed
            );
            out.push_str(fmt.bar_color);
            out.push('\n');
            lines += 1;
        }
        // return number of lines
        lines
    }

    fn format_self(
        &mut self,
        fmt: &mut BarFormatter<'_, '_, '_>,
        mut width: usize,
        state: &StateImmut,
    ) {
        use std::fmt::Write as _;
        let out = &mut *fmt.out;
        let temp = &mut *fmt.temp;

        // not enough width
        match width {
            0 => return,
            1 => {
                out.push('.');
                return;
            }
            2 => {
                out.push_str("..");
                return;
            }
            3 => {
                out.push_str("...");
                return;
            }
            4 => {
                out.push_str("[..]");
                return;
            }
            _ => {}
        }
        let (current, total) = self.real_current_total(state.unbounded);
        // --
        let show_current_total = !state.unbounded;
        let show_prefix = !state.prefix.is_empty();
        // -- :
        let show_percentage = state.show_percentage && total.is_some();
        let eta = self.estimate_remaining(state.unbounded, fmt.now, fmt.tick);
        let show_eta = eta.is_some();
        let show_message = !self.message.is_empty();

        struct Spacing {
            show_separator: bool,
            show_space_before_eta: bool,
            show_space_before_message: bool,
        }

        let spacing = if state.display_bytes {
            Spacing {
                show_separator: show_prefix
                    && (show_current_total || show_percentage || show_eta || show_message),
                show_space_before_eta: show_percentage || show_current_total,
                show_space_before_message: show_percentage || show_current_total || show_eta,
            }
        } else {
            Spacing {
                show_separator: show_prefix && (show_percentage || show_eta || show_message),
                show_space_before_eta: show_percentage,
                show_space_before_message: show_percentage || show_eta,
            }
        };

        if !state.display_bytes && show_current_total {
            temp.clear();
            // _: fmt for string does not fail
            let _ = match total {
                None => write!(temp, "{current}/?"),
                Some(total) => write!(temp, "{current}/{total}"),
            };

            // .len() is safe because / and numbers have the same byte size and width
            // -2 is safe because width > 4 here
            width -= 2;
            out.push('[');
            if temp.len() > width {
                // not enough space
                for _ in 0..width {
                    out.push('.');
                }
                out.push(']');
                return;
            }

            width -= temp.len();
            out.push_str(temp);
            out.push(']');
        }

        if width > 0 {
            out.push(' ');
            width -= 1;
        }

        if show_prefix {
            width = format_message_with_width(out, width, &state.prefix);
        }

        if spacing.show_separator && width > 2 {
            width -= 2;
            out.push_str(": ");
        }

        if state.display_bytes && show_current_total {
            temp.clear();
            // _: fmt for string does not fail
            let _ = match total {
                None => write!(temp, "{}", ByteFormat(current)),
                Some(total) => write!(temp, "{} / {}", ByteFormat(current), ByteFormat(total)),
            };

            if width >= temp.len() {
                width -= temp.len();
                out.push_str(temp);
            }

            if width > 0 {
                out.push(' ');
                width -= 1;
            }
        }

        if show_percentage {
            // unwrap: total is always Some
            let total = total.unwrap();
            if current == total {
                if width >= 4 {
                    width -= 4;
                    out.push_str("100%")
                }
            } else {
                let percentage = current as f32 * 100f32 / total as f32;
                temp.clear();
                // _: fmt for string does not fail
                let _ = write!(temp, "{percentage:.2}%");
                if width >= temp.len() {
                    width -= temp.len();
                    out.push_str(temp);
                }
            }
        }

        if let Some(eta) = eta {
            // ETA SS.SSs
            if spacing.show_space_before_eta && width > 0 {
                out.push(' ');
                width -= 1;
            }
            temp.clear();
            // _: fmt for string does not fail
            let _ = write!(temp, "ETA {eta:.2}s;");
            if width >= temp.len() {
                width -= temp.len();
                out.push_str(temp);
            }
        }

        if show_message {
            if spacing.show_space_before_message && width > 0 {
                out.push(' ');
                width -= 1;
            }
            format_message_with_width(out, width, &self.message);
        }
    }

    fn format_finish_message(&self, message: &str, unbounded: bool, in_bytes: bool) -> String {
        if unbounded {
            return message.to_string();
        }
        let (current, total) = self.real_current_total(unbounded);
        match (total, in_bytes) {
            (None, false) => {
                format!("[{current}/?] {message}")
            }
            (None, true) => {
                let current = ByteFormat(current);
                format!("{message} ({current})")
            }
            (Some(total), false) => {
                format!("[{current}/{total}] {message}")
            }
            (Some(total), true) => {
                let current = ByteFormat(current);
                let total = ByteFormat(total);
                format!("{message} ({current} / {total})")
            }
        }
    }
}

fn format_message_with_width(out: &mut String, mut width: usize, message: &str) -> usize {
    for (c, w) in ansi::with_width(message.chars()) {
        if w > width {
            break;
        }
        width -= w;
        out.push(c);
    }
    width
}

enum ChildState {
    /// The done message (if `keep` is true)
    Done(String),
    /// The interrupted message
    Interrupted(String),
    /// Still running
    Progress(usize, Weak<ProgressBar>),
}
impl ChildState {
    fn upgrade(&self) -> Option<ChildStateStrong<'_>> {
        Some(match self {
            ChildState::Done(x) => ChildStateStrong::Done(x),
            ChildState::Interrupted(x) => ChildStateStrong::Interrupted(x),
            ChildState::Progress(_, weak) => ChildStateStrong::Progress(weak.upgrade()?),
        })
    }
}

enum ChildStateStrong<'a> {
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

struct ByteFormat(u64);
impl std::fmt::Display for ByteFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (unit_bytes, unit_char) in [
            (1000_000_000_000, 'T'),
            (1000_000_000, 'G'),
            (1000_000, 'M'),
            (1000, 'k'),
        ] {
            if self.0 >= unit_bytes {
                let whole = self.0 / unit_bytes;
                let deci = (self.0 % unit_bytes) * 10 / unit_bytes;
                return write!(f, "{whole}.{deci}{unit_char}");
            }
        }
        write!(f, "{}B", self.0)
    }
}
