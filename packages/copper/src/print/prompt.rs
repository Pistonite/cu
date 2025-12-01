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

/// Show a password prompt and loops until a legal password is accepted.
///
/// Use this when prompting the user to set a password.
///
/// The console will have inputs hidden while user types, and the returned
/// value is a [`ZeroWhenDropString`](crate::ZeroWhenDropString)
///
/// Legal password must be non-empty, and contains only alphanumeric characters, or selected ascii
/// special characters.
///
/// ```rust,ignore
/// let password = cu::prompt_legal_password!("please enter your password")?;
/// cu::info!("user entered: {password}");
/// ```
#[cfg(feature = "prompt-password")]
#[macro_export]
macro_rules! prompt_legal_password {
    ($($fmt_args:tt)*) => {{
        loop {
            let p = $crate::__priv::__prompt(format_args!($($fmt_args)*), true)?;
            match $crate::check_password_legality(&*p) {
                Ok(()) => break $crate::Ok(p),
                Err(e) => {
                    $crate::error!("{e}");
                    ::std::mem::drop(p);
                }
            }
        }
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

pub fn __prompt(
    message: std::fmt::Arguments<'_>,
    is_password: bool,
) -> crate::Result<crate::ZeroWhenDropString> {
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

/// A string that will have its inner buffer zeroed when dropped
#[derive(Default, Clone)]
pub struct ZeroWhenDropString(String);
impl std::fmt::Display for ZeroWhenDropString {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<String> for ZeroWhenDropString {
    #[inline(always)]
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl Drop for ZeroWhenDropString {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: we don't use the string again
        for c in unsafe { self.0.as_bytes_mut() } {
            // SAFETY: c is a valid u8 pointer
            unsafe { std::ptr::write_volatile(c, 0) };
        }
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}
impl std::ops::Deref for ZeroWhenDropString {
    type Target = String;
    #[inline(always)]
    fn deref(&self) -> &String {
        &self.0
    }
}
impl std::ops::DerefMut for ZeroWhenDropString {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
