use crate::{Atomic, Context as _};

use crate::lv;
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
        $crate::__priv::__prompt(format_args!($($fmt_args)*), false).map(|x| x.to_string())
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
        $crate::__priv::__prompt_yesno(format_args!($($fmt_args)*))
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
        $crate::__priv::__prompt(format_args!($($fmt_args)*), true)
    }}
}

pub(crate) static PROMPT_LEVEL: Atomic<u8, lv::Prompt> =
    Atomic::new_u8(lv::Prompt::Interactive as u8);

pub fn __prompt_yesno(
    message: std::fmt::Arguments<'_>,
) -> crate::Result<bool> {
    match check_prompt_level(true) {
        Ok(false) => {},
        other => return other,
    };
    let mut answer = false;
    prompt_with_validation_impl(format_args!("{message} [y/n]"), false, |x| {
        x.make_ascii_lowercase();
        match x.trim() {
            "y" | "yes" => {
                answer = true;
                Ok(true)
            } 
            "n" | "no"  => {
                answer = false;
                Ok(true)
            }
            _ => {
                crate::hint!("please enter yes or no");
                Ok(false)
            }
        }
    })?;
    Ok(answer)
}

pub fn __prompt(
    message: std::fmt::Arguments<'_>,
    is_password: bool,
) -> crate::Result<crate::ZeroWhenDropString> {
    check_prompt_level(false)?;
    prompt_impl(&format!("{message}"), is_password)
}

fn prompt_with_validation_impl<F: FnMut(&mut String) -> crate::Result<bool>>(
    message: std::fmt::Arguments<'_>,
    is_password: bool,
    mut validator: F
) -> crate::Result<crate::ZeroWhenDropString> {
    let message = format!("{message}");
    loop {
        let mut result = prompt_impl(&message, is_password)?;
        if validator(&mut result)? {
            return Ok(result);
        }
    }
}

fn prompt_impl(
    message: &str,
    is_password: bool,
) -> crate::Result<crate::ZeroWhenDropString> {
    let recv = {
        let Ok(mut printer) = super::PRINTER.lock() else {
            crate::bail!("prompt failed: global print lock poisoned");
        };
        printer.show_prompt(message, is_password)
    };
    let result = crate::check!(recv.recv(), "error while showing prompt")?;
    crate::check!(result, "io error while showing prompt")
}

// Ok(true) -> answer Yes
// Ok(false) -> prompt
// Err -> bail
fn check_prompt_level(is_yesno: bool) -> crate::Result<bool> {
    if is_yesno {
        match PROMPT_LEVEL.get() {
            // do not even show the prompt if --yes
            lv::Prompt::YesOrInteractive | lv::Prompt::YesOrBlock => return Ok(true),
            lv::Prompt::Interactive => return Ok(false),
            lv::Prompt::Block => { }
        }
    } else {
        if !matches!(PROMPT_LEVEL.get(), lv::Prompt::YesOrBlock | lv::Prompt::Block) {
            return Ok(false);
        }
    }
    crate::bail!("prompt not allowed with --non-interactive");
}
