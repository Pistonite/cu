use crate::{Atomic, Context as _};

use super::PromptLevel;
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
#[cfg(feature = "prompt")]
#[macro_export]
macro_rules! prompt {
    ($($fmt_args:tt)*) => {{
        $crate::__priv::__prompt(format_args!($($fmt_args)*))
    }}
}
pub(crate) static PROMPT_LEVEL: Atomic<u8, PromptLevel> =
    Atomic::new_u8(PromptLevel::Interactive as u8);

pub fn __prompt_yesno(message: std::fmt::Arguments<'_>) -> crate::Result<bool> {
    match PROMPT_LEVEL.get() {
        PromptLevel::Interactive => {}
        PromptLevel::Yes => return Ok(true),
        PromptLevel::No => {
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
            printer.show_prompt(&message)
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

pub fn __prompt(message: std::fmt::Arguments<'_>) -> crate::Result<String> {
    if let PromptLevel::No = PROMPT_LEVEL.get() {
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
            printer.show_prompt(&message)
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
