use cu::cli::printer::PRINTER;
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
