use std::sync::Arc;
use std::time::Instant;

use crate::print::progress::bar::ProgressBar;
use crate::print::progress::eta::Estimater;
use crate::print::{Tick, ansi};

/// Immutable part of progress bar
pub struct StateImmut {
    /// Parent of this bar
    parent: Arc<ProgressBar>,
    /// The prefix message (corresponds to message in the builder)
    prefix: String,
    /// None means don't keep the progress bar printed
    /// (the default done message is formatted at spawn time)
    done_message: Option<String>,
    /// None means use the default
    interrupted_message: Option<String>,
    /// If percentage field is shown
    show_percentage: bool,
    /// If the steps are unbounded
    pub unbounded: bool,
    /// Display the progress using bytes format
    display_bytes: bool,
}

pub struct State {
    unreal_total: u64,
    unreal_current: u64,
    message: String,
    eta: Option<Estimater>
}
impl State {
    pub fn new(total: u64, message: String, eta: Option<Estimater>) -> Self {
        Self {
            unreal_total: total,
            unreal_current: 0,
            message,
            eta,
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
        self.eta.as_mut()?
        .update(now, self.unreal_current.min(self.unreal_total)
            , self.unreal_total, tick
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
            (self.unreal_current.min(self.unreal_total), Some(self.unreal_total))
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

    pub fn set_message(&mut self, message: &str) {
        self.message.clear();
        self.message.push_str(message);
    }


    pub fn format(
        &mut self,
        mut width: usize,
        now: &mut Option<Instant>,
        tick: Tick,
        out: &mut String,
        temp: &mut String,
        state: &StateImmut,
    ) {
        use std::fmt::Write as _;

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
        let eta = self.estimate_remaining(state.unbounded, now, tick);
        let show_eta = eta.is_some();
        let show_message = !self.message.is_empty();

        struct Spacing {
            show_separator: bool,
            show_space_before_eta: bool,
            show_space_before_message: bool
        }

        let spacing = if state.display_bytes {
            Spacing {
                show_separator: show_prefix && (show_current_total || show_percentage || show_eta || show_message),
                show_space_before_eta: show_percentage || show_current_total,
                show_space_before_message: show_percentage ||show_current_total ||show_eta,
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

            if width > 0 {
                out.push(' ');
                width -= 1;
            }
        }

        if show_prefix {
            for (c, w) in ansi::with_width(state.prefix.chars()) {
                if w > width {
                    break;
                }
                width -= w;
                out.push(c);
            }
        }

        if spacing.show_separator && width > 2{
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
            for (c, w) in ansi::with_width(self.message.chars()) {
                if w > width {
                    break;
                }
                width -= w;
                out.push(c);
            }
        }
    }
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
                let deci = (self.0 % unit_bytes) * 10/unit_bytes;
                return write!(f, "{whole}.{deci}{unit_char}");
            }
        }
        write!(f, "{}B", self.0)
    }
}
