use crate::cli::fmt::{self, ansi};

/// Buffer for formatting printing messages
pub(crate) struct FormatBuffer {
    /// Total width to print
    width: usize,
    /// Current char position in the line
    curr: usize,
    /// Internal buffer
    buffer: String,
    /// ANSI code for gray
    gray_color: &'static str,
    /// ANSI code for the current text color
    text_color: &'static str,
    /// Process ansi escape in the formatting
    escape_state: FormatBufferEscapeState,
}

impl FormatBuffer {
    pub fn new() -> Self {
        Self {
            width: fmt::term_width_or_max(),
            curr: 0,
            buffer: String::new(),
            gray_color: "",
            text_color: "",
            escape_state: FormatBufferEscapeState::None,
        }
    }
    /// Get the formatted buffer content
    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
    /// Take the formatted buffer content out, leaving empty string
    #[allow(unused)] // prompt uses it
    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }
    /// Reset
    pub fn reset(&mut self, gray_color: &'static str, text_color: &'static str) {
        self.curr = 0;
        self.buffer.clear();
        self.width = fmt::term_width_or_max();
        self.gray_color = gray_color;
        self.text_color = text_color;
        self.escape_state = FormatBufferEscapeState::None;
    }

    /// Push a newline character (note this is different from [`new_line`](Self::new_line))
    pub fn push_lf(&mut self) {
        self.buffer.push('\n');
    }
    /// Push a string as control characters. i.e. the content will be
    /// appended to the buffer without formatting.
    pub fn push_control(&mut self, x: &str) {
        self.buffer.push_str(x)
    }
    /// Push and format string content
    pub fn push_str(&mut self, x: &str) {
        for (c, w) in ansi::with_width(x.chars()) {
            self.push(c, w);
        }
    }
    /// Push a character with its width
    pub fn push(&mut self, c: char, w: usize) {
        if c == '\n' {
            self.new_line();
            return;
        }
        if self.width < 5 {
            // window width is too small to meaningfully format
            self.buffer.push(c);
            return;
        }
        if w < self.width && self.curr > self.width - w {
            self.new_line();
        }
        self.buffer.push(c);
        match self.escape_state {
            FormatBufferEscapeState::None => {
                if c == '\x1b' {
                    self.escape_state = FormatBufferEscapeState::SawEsc;
                }
            }
            FormatBufferEscapeState::SawEsc => {
                if c == '[' {
                    self.escape_state = FormatBufferEscapeState::SawBracket;
                } else {
                    self.escape_state = FormatBufferEscapeState::None;
                }
            }
            FormatBufferEscapeState::SawBracket => {
                if c.is_ascii_digit() {
                    let code = c as u8 - b'0';
                    self.escape_state = FormatBufferEscapeState::SawCode(code as u32);
                } else if c == ';' {
                    self.escape_state = FormatBufferEscapeState::SawBracket;
                } else {
                    self.escape_state = FormatBufferEscapeState::None;
                }
            }
            FormatBufferEscapeState::SawCode(code) => {
                if c.is_ascii_digit() {
                    let digit = c as u8 - b'0';
                    self.escape_state = FormatBufferEscapeState::SawCode(code * 10 + digit as u32);
                } else if c == ';' || c == 'm' {
                    // if resetting foreground or full, push text color again
                    let need_reset = code == 0 || code == 39;
                    if c == ';' {
                        if need_reset {
                            // terminate the control sequence first
                            // we just pushed `c` so pop is safe
                            self.buffer.pop();
                            self.buffer.push('m');
                            // start new control sequence
                            self.buffer.push_str(self.text_color);
                            // continue control sequence
                            self.buffer.push_str("\x1b[");
                        }
                        self.escape_state = FormatBufferEscapeState::SawBracket;
                    } else {
                        if need_reset {
                            self.buffer.push_str(self.text_color);
                        }
                        self.escape_state = FormatBufferEscapeState::None;
                    }
                } else {
                    self.escape_state = FormatBufferEscapeState::None;
                }
            }
        }
        self.curr += w;
    }
    /// Start formatting a new line
    pub fn new_line(&mut self) {
        self.buffer.push('\n');
        self.buffer.push_str(self.gray_color);
        self.buffer.push_str(" | ");
        self.buffer.push_str(self.text_color);
        self.curr = 3;
    }
}

enum FormatBufferEscapeState {
    None,
    SawEsc,
    SawBracket,
    SawCode(u32),
}
