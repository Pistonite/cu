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
    if cfg!(feature = "__test") {
        // fix the size in test
        Some((60, 20))
    } else {
        use terminal_size::*;
        terminal_size().map(|(Width(w), Height(h))| ((w as usize).min(400), (h as usize).min(400)))
    }
}
