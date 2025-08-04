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
    yellow: "\x1b[1;33m",
    red: "\x1b[1;31m",
    gray: "\x1b[1;30m",
    magenta: "\x1b[1;35m",
    cyan: "\x1b[1;36m",
    green: "\x1b[1;32m",
};

#[inline]
pub(crate) const fn colors(use_color: bool) -> Colors {
    if use_color { COLOR } else { NOCOLOR }
}

/// Control codes definition
#[derive(Clone, Copy)]
pub(crate) struct Controls {
    pub move_to_begin_and_clear: &'static str,
}

static NOCONTROL: Controls = Controls {
    move_to_begin_and_clear: "",
};

static CONTROL: Controls = Controls {
    move_to_begin_and_clear: "\r\x1b[K",
};

#[inline]
pub(crate) const fn controls(use_controls: bool) -> Controls {
    if use_controls { CONTROL } else { NOCONTROL }
}

/// Iterator of (char, width)
pub(crate) fn with_width(x: std::str::Chars<'_>) -> AnsiWidthIter<'_> {
    AnsiWidthIter {
        is_escaping: false,
        chars: x,
    }
}

pub(crate) struct AnsiWidthIter<'a> {
    is_escaping: bool,
    chars: std::str::Chars<'a>,
}

impl<'a> Iterator for AnsiWidthIter<'a> {
    type Item = (char, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.chars.next()?;
        let width = if self.is_escaping {
            if is_ansi_end_char(c) {
                self.is_escaping = false;
            }
            0
        } else if c == '\x1b' {
            self.is_escaping = true;
            0
        } else {
            use unicode_width::UnicodeWidthChar;
            c.width_cjk().unwrap_or(0)
        };

        Some((c, width))
    }
}

pub(crate) fn is_ansi_end_char(c: char) -> bool {
    // we only do very basic check right now
    c < u8::MAX as char && b"mAKGJBCDEFHSTfhlin".contains(&(c as u8))
}
