use cu::cli::printer::{PRINTER, PrintPayload};
use cu::lv;
/// Print something
///
/// This is similar to `info`, but unlike info, this message will still log with `-q`.
#[macro_export]
#[cfg(feature = "print")]
macro_rules! print {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__print_with_level($crate::lv::P, format_args!($($fmt_args)*));
    }}
}
/// Logs a hint message
#[macro_export]
#[cfg(feature = "print")]
macro_rules! hint {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__print_with_level($crate::lv::H, format_args!($($fmt_args)*));
    }}
}

/// Print message with a custom log format and level.
///
/// This lets you customize the "level char", the separator char, and
/// the colors for level char, separator char, and the main text color.
/// Note that if colors are disabled from CLI (--color=never), the control
/// sequences are replaced with empty string.
///
/// ```rust
/// # use pistonite_cu as cu;
/// static GREEN: &str = "\x1b[92m";
/// static RED: &str = "\x1b[91m";
/// static YELLOW: &str = "\x1b[93m";
/// static RESET: &str = "\x1b[0m";
///
/// /// This logs P) in green
/// macro_rules! pass {
///    ($($args:tt)*) => { cu::custom_print!(cu::lv::I, GREEN, 'P', "", ')', GREEN, $($args)*) }
/// }
///
/// /// This logs F) in red
/// macro_rules! fail {
///    ($($args:tt)*) => { cu::custom_print!(cu::lv::E, RED, 'F', "", ')', RED, $($args)*) }
/// }
///
/// let count = 5;
/// pass!("{count} tests passed");
/// fail!("{count} tests failed");
/// // the part after reset remains red
/// fail!("{YELLOW}some{RESET} tests failed");
/// ```
#[macro_export]
#[cfg(feature = "print")]
macro_rules! custom_print {
    (
        $level:expr, $level_char_control:expr, $level_char:expr,
        $sep_char_control:expr, $sep_char:expr, $text_control:expr,
        $($fmt_args:tt)*
    ) => {{
        $crate::cli::__print_payload_with_level(
            $level,
            $level_char_control,
            $level_char,
            $sep_char_control,
            $sep_char,
            $text_control,
            format_args!($($fmt_args)*)
        );
    }};
}

/// Show prompt to the user. See [Prompting](fn@crate::prompt)
#[cfg(all(feature = "prompt", not(feature = "coroutine")))]
#[macro_export]
macro_rules! prompt {
    ($($fmt_args:tt)*) => {{
        $crate::cli::prompt(format!($($fmt_args)*)).or_cancel().run()
    }};
}

/// Show prompt to the user. See [Prompting](fn@crate::prompt)
#[cfg(all(feature = "prompt", feature = "coroutine"))]
#[macro_export]
macro_rules! prompt {
    (async $($fmt_args:tt)*) => {{
        $crate::cli::prompt(format!($($fmt_args)*)).or_cancel().co_run().await
        }};
    ($($fmt_args:tt)*) => {{
        $crate::cli::prompt(format!($($fmt_args)*)).or_cancel().run()
    }};
}

/// Show a Yes/No prompt. See [Prompting](fn@crate::prompt)
#[cfg(all(feature = "prompt", not(feature = "coroutine")))]
#[macro_export]
macro_rules! yesno {
    ($($fmt_args:tt)*) => {{
        $crate::cli::yesno(format!($($fmt_args)*)).run()
    }};
}

/// Show a Yes/No prompt. See [Prompting](fn@crate::prompt)
#[cfg(all(feature = "prompt", feature = "coroutine"))]
#[macro_export]
macro_rules! yesno {
    (async $($fmt_args:tt)*) => {{
        $crate::cli::yesno(format!($($fmt_args)*)).co_run().await
        }};
    ($($fmt_args:tt)*) => {{
        $crate::cli::yesno(format!($($fmt_args)*)).run()
    }};
}

/// Internal print function for macros
#[doc(hidden)]
pub fn __print_with_level(lv: lv::Lv, message: std::fmt::Arguments<'_>) {
    if !lv.can_print(lv::PRINT_LEVEL.get()) {
        return;
    }
    let message = format!("{message}");
    if let Ok(mut printer) = PRINTER.lock() {
        if let Some(printer) = printer.as_mut() {
            printer.print_message(lv, &message);
        }
    }
}

/// Internal print function for custom print formats
#[doc(hidden)]
pub fn __print_payload_with_level(
    lv: lv::Lv,
    mut level_char_control: &'static str,
    level_char: char,
    mut sep_char_control: &'static str,
    sep_char: char,
    mut text_control: &'static str,
    message: std::fmt::Arguments<'_>,
) {
    if !lv.can_print(lv::PRINT_LEVEL.get()) {
        return;
    }
    if !lv::color_enabled() {
        level_char_control = "";
        sep_char_control = "";
        text_control = "";
    }
    let message = format!("{message}");
    let payload = PrintPayload {
        level_char_control,
        level_char,
        sep_char_control,
        sep_char,
        text_control,
        message: &message,
    };
    if let Ok(mut printer) = PRINTER.lock() {
        if let Some(printer) = printer.as_mut() {
            printer.print_payload(payload);
        }
    }
}
