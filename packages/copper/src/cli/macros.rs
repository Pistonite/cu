
use cu::lv;
use cu::cli::printer::PRINTER;
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

/// Show a prompt
///
/// Use the `prompt-password` feature and [`prompt_password!`](crate::prompt_password) macro
/// if prompting for a password, which will hide user's input from the console
///
/// ```rust,ignore
/// let name = cu::prompt!("please enter your name")?;
/// cu::info!("user entered: {name}");
/// ```
#[cfg(feature = "prompt")]
#[macro_export]
macro_rules! prompt {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__prompt(format_args!($($fmt_args)*), false).map(|x| x.to_string())
    }}
}

/// Show a Yes/No prompt
///
/// Return `true` if the answer is Yes. Return an error if prompt is not allowed
/// ```rust,ignore
/// if cu::yesno!("do you want to continue?")? {
///     cu::info!("user picked yes");
/// }
/// ```
#[cfg(feature = "prompt")]
#[macro_export]
macro_rules! yesno {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__prompt_yesno(format_args!($($fmt_args)*))
    }}
}


/// Show a password prompt
///
/// The console will have inputs hidden while user types, and the returned
/// value is a [`ZeroWhenDropString`](crate::ZeroWhenDropString)
///
/// ```rust,ignore
/// let password = cu::prompt_password!("please enter your password")?;
/// cu::info!("user entered: {password}");
/// ```
#[cfg(feature = "prompt-password")]
#[macro_export]
macro_rules! prompt_password {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__prompt(format_args!($($fmt_args)*), true)
    }}
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
