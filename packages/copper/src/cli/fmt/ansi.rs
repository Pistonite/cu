/// Color code definition
#[derive(Clone, Copy)]
pub(crate) struct Colors {
    pub reset: &'static str,
    pub yellow: &'static str,
    pub red: &'static str,
    pub gray: &'static str,
    pub magenta: &'static str,
    pub cyan: &'static str,
    pub green: &'static str,
}

static NOCOLOR: Colors = Colors {
    reset: "",
    yellow: "",
    red: "",
    gray: "",
    magenta: "",
    cyan: "",
    green: "",
};

static COLOR: Colors = Colors {
    reset: "\x1b[0m",
    yellow: "\x1b[93m",
    red: "\x1b[91m",
    gray: "\x1b[90m",
    magenta: "\x1b[95m",
    cyan: "\x1b[96m",
    green: "\x1b[92m",
};

#[inline]
pub(crate) const fn colors(use_color: bool) -> Colors {
    if use_color { COLOR } else { NOCOLOR }
}

/// Iterator of (char, width)
pub(crate) fn with_width(x: std::str::Chars<'_>) -> AnsiWidthIter<'_> {
    AnsiWidthIter {
        state: AnsiEscapeState::None,
        chars: x,
    }
}

pub(crate) struct AnsiWidthIter<'a> {
    state: AnsiEscapeState,
    chars: std::str::Chars<'a>,
}

impl<'a> Iterator for AnsiWidthIter<'a> {
    type Item = (char, usize);

    fn next(&mut self) -> Option<Self::Item> {
        use unicode_width::UnicodeWidthChar;
        let c = self.chars.next()?;
        let width = match self.state {
            AnsiEscapeState::None => {
                if c == '\x1b' {
                    self.state = AnsiEscapeState::SawEsc;
                    0
                } else {
                    c.width_cjk().unwrap_or(0)
                }
            }
            AnsiEscapeState::SawEsc => match c {
                '[' => {
                    self.state = AnsiEscapeState::EscapingControl;
                    0
                }
                ']' => {
                    self.state = AnsiEscapeState::EscapingOsCommand;
                    0
                }
                _ => {
                    self.state = AnsiEscapeState::None;
                    c.width_cjk().unwrap_or(0)
                }
            },
            AnsiEscapeState::EscapingControl => {
                if is_esc_end(c) {
                    self.state = AnsiEscapeState::None;
                }
                0
            }
            AnsiEscapeState::EscapingOsCommand => {
                match c {
                    '\x1b' => {
                        self.state = AnsiEscapeState::EscapingOsCommandSawEsc;
                    }
                    '\x07' => {
                        self.state = AnsiEscapeState::None;
                    }
                    _ => {}
                }
                0
            }
            AnsiEscapeState::EscapingOsCommandSawEsc => {
                if c == '\\' {
                    self.state = AnsiEscapeState::None;
                } else {
                    self.state = AnsiEscapeState::EscapingOsCommand;
                }
                0
            }
        };

        Some((c, width))
    }
}

pub(crate) fn is_esc_end(c: char) -> bool {
    // we only do very basic check right now
    c < u8::MAX as char && b"mAKGJBCDEFHSTfhlin\\\x07".contains(&(c as u8))
}

enum AnsiEscapeState {
    None,
    SawEsc,
    EscapingControl,
    EscapingOsCommand,
    EscapingOsCommandSawEsc,
}
