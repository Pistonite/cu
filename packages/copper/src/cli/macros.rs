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

/// # Prompting
/// The `prompt` feature allows displaying prompts in the console to accept
/// user input. The prompts are thread-safe and synchronized with the printer.
/// When a prompt is active, outputs to the console will be buffered inside the printer.
/// Progress bars will also be paused.
///
/// Prompts are driven by macros where you can format a prompt message
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let name = cu::prompt!("please enter your name")?;
/// cu::info!("user entered: {name}");
/// # cu::Ok(())
/// ```
///
/// if the prompt is for password, use the `prompt-password` feature (which implies `prompt`)
/// to enable `prompt_password!` macro.
///
/// See other macros for advanced usage:
/// - [`cu::yesno!`](macro@crate::yesno): Display a `[y/n]` prompt which loops
///   until user chooses yes or no.
/// - [`cu::prompt_password!`](macro@crate::prompt_password):
///   Display a prompt where the input will be hidden.
/// - [`cu::prompt_validate!`](macro@crate::prompt_validate) (and
///   [`prompt_password_validate!`](macro@crate::prompt_password_validate)) to loop the prompt until a validation function passes.
///
/// # Multiple Prompts
/// If multiple prompts are requested (for example from many threads), they are put into a FIFO
/// queue. Only one prompt will be displayed to the user at a time.
///
/// # Interaction with Progress Bars
/// If there are both progress bars and a prompt active, the prompt is displayed below
/// the progress bars. This lines up with the common expectation that prompts are shown
/// at the bottom of the output. Progress bars will also be paused - otherwise the user
/// will be typing their answer all over in the animated area.
///
/// This brings one interesting use case, which is printing other messages related
/// to the prompt before the prompt. For example:
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// // ... imagine a progress bar is active here
/// cu::warn!(
///     "this is an important warning about the prompt! read this carefully when answering!"
/// );
/// let _ = cu::prompt!("please enter important information")?;
/// # cu::Ok(())
/// ```
///
/// The warning message is guaranteed to be printed before the prompt. However, because
/// there are progress bars active, the warning message and the prompt will be separated
/// by the progress bar. The user might miss the important message.
///
/// ```text
/// W] this is an important warning about the prompt! read this carefully when answering!
///  ][42/100] progress running...
/// !] please enter important information
/// -:
/// ```
///
/// The way to address this is to print any message related to the prompt within the prompt
/// macro:
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// // ... imagine a progress bar is active here
/// let _ = cu::prompt!(r"this is an important warning about the prompt! read this carefully when answering!
/// please enter important information")?;
/// # cu::Ok(())
/// ```
/// output:
/// ```text
///  ][42/100] progress running...
/// !] this is an important warning about the prompt! read this carefully when answering!
///  | please enter important information
/// -:
/// ```
///
#[cfg(feature = "prompt")]
#[macro_export]
macro_rules! prompt {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__prompt(format_args!($($fmt_args)*), false).map(|x| x.to_string())
    }}
}

/// Show a Yes/No prompt
///
/// Return `true` if the answer is Yes. Return an error if prompt is not allowed.
///
/// If `-y` is specified from the command line, then the prompt will not show,
/// and `true` will be returned immediately.
///
/// If user does not answer `y` or `n`, the prompt will show again, until
/// user makes a decision.
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// if cu::yesno!("do you want to continue?")? {
///     cu::info!("user picked yes");
/// }
/// # cu::Ok(())
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
/// value is a [`cu::ZString`](struct@crate::ZString)
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let password = cu::prompt_password!("please enter your password")?;
/// cu::info!("user entered: {password}");
/// # cu::Ok(())
/// ```
#[cfg(feature = "prompt-password")]
#[macro_export]
macro_rules! prompt_password {
    ($($fmt_args:tt)*) => {{
        $crate::cli::__prompt(format_args!($($fmt_args)*), true)
    }}
}

/// Loop a prompt until a validation function passes
///
/// The validation function takes a `&mut String`,
/// and returns `cu::Result<bool>`, where:
/// - `Ok(true)` means the validation passed.
/// - `Ok(false)` means the validation failed. The function can optionally
///   print some kind of error or hint message
/// - `Err` means there is an error, the error will be propagated to the prompt call.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// // note that extra parenthesis is needed if the format args
/// // are not inlined into the formatting literal
/// let expected = "rust";
/// let answer = cu::prompt_validate!(
///     ("what's your favorite programming language? please answer {}", expected),
///     |answer| {
///         if answer == expected {
///             return Ok(true);
///         }
///         if answer == "javascript" {
///             cu::bail!("that's not good");
///         }
///         cu::error!("try again");
///         Ok(false)
///     }
/// )?;
/// assert!(answer == expected);
/// # cu::Ok(())
/// ```
///
/// The validation function can be a `FnMut` closure, which means
/// it can double as a result parsing function if needed
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let mut index: i32 = 0;
/// cu::prompt_validate!(
///     "select a number between 0 and 5",
///     |answer| {
///         let number = match cu::parse::<i32>(answer) {
///             Err(e) => {
///                 cu::error!("{e}");
///                 cu::hint!("please ensure you are entering a number");
///                 return Ok(false);
///             }
///             Ok(x) => x
///         };
///         if number < 0 {
///             cu::error!("the number you entered is too small");
///             return Ok(false);
///         }
///         if number > 5 {
///             cu::error!("the number you entered is too big");
///             return Ok(false);
///         }
///         index = number;
///         Ok(true)
///     }
/// )?;
/// cu::info!("index is {index}");
/// # cu::Ok(())
/// ```
///
/// For the password version, see [`prompt_password_validate`](crate::prompt_password_validate)
///
#[cfg(feature = "prompt")]
#[macro_export]
macro_rules! prompt_validate {
    ($l:literal, $validator:expr) => {{
        $crate::cli::__prompt_with_validation(format_args!($l), false, $validator)
        .map(|x| x.to_string())
    }};
    (($($fmt_args:tt)*), $validator:expr) => {{
        $crate::cli::__prompt_with_validation(format_args!($($fmt_args)*), false, $validator)
        .map(|x| x.to_string())
    }}
}

/// Loop a password prompt until a validation function passes
///
/// The validation function takes a `&mut String`,
/// and returns `cu::Result<bool>`, where:
/// - `Ok(true)` means the validation passed.
/// - `Ok(false)` means the validation failed. The function can optionally
///   print some kind of error or hint message
/// - `Err` means there is an error, the error will be propagated to the prompt call.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// // note that extra parenthesis is needed if the format args
/// // are not inlined into the formatting literal
/// let password = cu::prompt_password_validate!(
///     "please enter a password between 8 and 16 charactres and only contain sensible characters",
///     |answer| {
///         if answer == "123456" {
///             cu::bail!("how can you do that, bye");
///         }
///         if answer.len() < 8 {
///             cu::error!("password is too short");
///             return Ok(false);
///         }
///         if answer.len() > 16 {
///             cu::error!("password is too long");
///             return Ok(false);
///         }
///         if let Err(e) = cu::password_chars_legal(answer) {
///             cu::error!("invalid password: {e}");
///             return Ok(false);
///         }
///         Ok(true)
///     }
/// )?;
/// cu::print!("{password}");
/// # cu::Ok(())
/// ```
#[cfg(feature = "prompt-password")]
#[macro_export]
macro_rules! prompt_password_validate {
    ($l:literal, $validator:expr) => {{
        $crate::cli::__prompt_with_validation(format_args!($l), true, $validator)
    }};
    (($($fmt_args:tt)*), $validator:expr) => {{
        $crate::cli::__prompt_with_validation(format_args!($($fmt_args)*), true, $validator)
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
