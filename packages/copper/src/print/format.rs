use super::ansi;

/// Format and invoke a print macro
///
/// # Example
/// ```rust
/// let x = cu::fmtand!(error!("found {} errors", 3));
/// assert_eq!(x, "found 3 errors");
/// ```
#[macro_export]
macro_rules! fmtand {
    ($mac:ident !( $($fmt_args:tt)* )) => {{
        let s = format!($($fmt_args)*);
        $crate::$mac!("{s}");
        s
    }}
}
/// Invoke a print macro, then bail with the same message
///
/// # Example
/// ```rust
/// # fn main() {
/// fn fn_1() -> cu::Result<()> {
///     cu::bailand!(error!("found {} errors", 3));
/// }
/// fn fn_2() -> cu::Result<()> {
///     cu::bailand!(warn!("warning!"));
/// }
/// assert!(fn_1().is_err()); // will also log error "found 3 errors"
/// assert!(fn_2().is_err()); // will also log warning "warning!"
/// # }
/// ```
#[macro_export]
macro_rules! bailand {
    ($mac:ident !( $($fmt_args:tt)* )) => {{
        let s = format!($($fmt_args)*);
        $crate::$mac!("{s}");
        $crate::bail!(s);
    }}
}
/// Invoke a print macro, then panic with the same message
///
/// # Example
/// ```rust,no_run
/// cu::panicand!(error!("found {} errors", 3));
/// ```
#[macro_export]
macro_rules! panicand {
    ($mac:ident !( $($fmt_args:tt)* )) => {{
        let s = format!($($fmt_args)*);
        $crate::$mac!("{s}");
        panic!("{s}");
    }}
}

/// Get the terminal width, or the internal max if cannot get 
pub fn term_width_or_max() -> usize {
    term_width().unwrap_or(400)
}

/// Get the terminal width, capped as some internal amount
pub fn term_width() -> Option<usize> {
    term_width_height().map(|x| x.0)
}

/// Get the terminal height, capped as some internal amount
pub fn term_width_height() -> Option<(usize, usize)> {
    use terminal_size::*;
    terminal_size().map(|(Width(w), Height(h))| ((w as usize).min(400), (h as usize).min(400)))
}

pub(crate) struct FormatBuffer {
    width: usize,
    curr: usize,
    buffer: String,
    gray_color: &'static str,
    text_color: &'static str,
}

impl FormatBuffer {
    pub fn new() -> Self {
        Self {
            width: term_width_or_max(),
            curr: 0,
            buffer: String::new(),
            gray_color: "",
            text_color: "",
        }
    }
    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }
    pub fn reset(&mut self, gray_color: &'static str, text_color: &'static str) {
        self.curr = 0;
        self.buffer.clear();
        self.width = term_width_or_max();
        self.gray_color = gray_color;
        self.text_color = text_color;
    }
    pub fn end(&mut self) {
        self.buffer.push('\n');
    }

    pub fn push_str(&mut self, x: &str) {
        for (c, w) in ansi::with_width(x.chars()) {
            self.push(c, w);
        }
    }
    pub fn push_control(&mut self, x: &str) {
        self.buffer.push_str(x)
    }
    pub fn push(&mut self, c: char, w: usize) {
        if c == '\n' {
            self.new_line();
            return;
        }
        if self.width < 5 {
            // give up
            self.buffer.push(c);
            return;
        }
        if w < self.width && self.curr > self.width - w {
            self.new_line();
        }
        self.buffer.push(c);
        self.curr += w;
    }

    pub fn new_line(&mut self) {
        self.buffer.push('\n');
        self.buffer.push_str(self.gray_color);
        self.buffer.push_str(" | ");
        self.buffer.push_str(self.text_color);
        self.curr = 3;
    }
}
