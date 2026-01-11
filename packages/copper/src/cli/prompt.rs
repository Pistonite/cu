
use crate::{Atomic, Context as _};
use crate::lv;
use crate::cli::printer::PRINTER;

pub(crate) static PROMPT_LEVEL: Atomic<u8, lv::Prompt> =
    Atomic::new_u8(lv::Prompt::Interactive as u8);

#[doc(hidden)]
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

#[doc(hidden)]
pub fn __prompt(
    message: std::fmt::Arguments<'_>,
    is_password: bool,
) -> cu::Result<cu::ZString> {
    check_prompt_level(false)?;
    prompt_impl(&format!("{message}"), is_password)
}

fn prompt_with_validation_impl<F: FnMut(&mut String) -> crate::Result<bool>>(
    message: std::fmt::Arguments<'_>,
    is_password: bool,
    mut validator: F
) -> cu::Result<cu::ZString> {
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
) -> cu::Result<cu::ZString> {
    let recv = {
        if let Ok(mut printer) = PRINTER.lock() && let Some(printer) = printer.as_mut() {
            printer.show_prompt(message, is_password)
        }  else {
            crate::bail!("prompt failed: failed to lock global printer");
        }
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
