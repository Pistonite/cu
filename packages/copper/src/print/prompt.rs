use crate::{Atomic, Context as _};

use crate::lv;
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

/// Show a prompt
///
/// Use the `prompt-password` feature and [`prompt_password!`] macro
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
        $crate::__priv::__prompt(format_args!($($fmt_args)*), false)
    }}
}

/// Show a password prompt
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

pub fn __prompt_yesno(message: std::fmt::Arguments<'_>) -> crate::Result<bool> {
    match PROMPT_LEVEL.get() {
        lv::Prompt::Interactive => {}
        lv::Prompt::Yes => return Ok(true),
        lv::Prompt::No => {
            crate::bailand!(error!(
                "prompt not allowed in non-interactive mode: {message}"
            ));
        }
    }

    let message = format!("{message} [y/n]");
    let _scope = PromptJoinScope;
    loop {
        let recv = {
            let Ok(mut printer) = super::PRINTER.lock() else {
                crate::bailand!(error!("prompt failed: global print lock poisoned"));
            };
            printer.show_prompt(&message, false)
        };
        let result = recv
            .recv()
            .with_context(|| format!("recv error while showing the prompt: {message}"))?;
        match result {
            Err(e) => {
                Err(e).context(format!("io error while showing the prompt: {message}"))?;
            }
            Ok(mut x) => {
                x.make_ascii_lowercase();
                match x.trim() {
                    "y" | "yes" => return Ok(true),
                    "n" | "no" => return Ok(false),
                    _ => {}
                }
            }
        }
        crate::error!("please enter yes or no");
    }
}

pub fn __prompt(message: std::fmt::Arguments<'_>, is_password: bool) -> crate::Result<String> {
    if let lv::Prompt::No = PROMPT_LEVEL.get() {
        crate::bailand!(error!(
            "prompt not allowed in non-interactive mode: {message}"
        ));
    }
    let message = format!("{message}");
    let result = {
        let _scope = PromptJoinScope;
        let recv = {
            let Ok(mut printer) = super::PRINTER.lock() else {
                crate::bailand!(error!("prompt failed: global print lock poisoned"));
            };
            printer.show_prompt(&message, is_password)
        };
        recv.recv()
            .with_context(|| format!("recv error while showing the prompt: {message}"))?
    };

    result.with_context(|| format!("io error while showing the prompt: {message}"))
}

struct PromptJoinScope;
impl Drop for PromptJoinScope {
    fn drop(&mut self) {
        let handle = {
            let Ok(mut printer) = super::PRINTER.lock() else {
                return;
            };
            let Some(handle) = printer.take_prompt_task_if_should_join() else {
                return;
            };
            handle
        };
        let _: Result<_, _> = handle.join();
    }
}
