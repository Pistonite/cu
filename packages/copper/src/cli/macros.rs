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

///
/// The `prompt` feature also enables the `prompt_password!` macro for password input.
///
/// See other macros for advanced usage:
/// - [`cu::yesno!`](macro@crate::yesno): Display a `[y/n]` prompt which loops
///   until user chooses yes or no.
/// - [`cu::prompt_password!`](macro@crate::prompt_password):
///   Display a prompt where the input will be hidden.
/// - [`cu::prompt_validate!`](macro@crate::prompt_validate) (and
///   [`prompt_password_validate!`](macro@crate::prompt_password_validate)) to loop the prompt until a validation function passes.
///
///
#[cfg(feature = "prompt")]
#[macro_export]
macro_rules! prompt {
    ($($fmt_args:tt)*) => {{
        $crate::cli::prompt(format!($($fmt_args)*)).run()
    }};
    (async $($fmt_args:tt)*) => {{
        $crate::cli::prompt(format!($($fmt_args)*)).co_run().await
    }};
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
        $crate::cli::yesno(format!($($fmt_args)*)).run()
    }}
}

// /// Show a password prompt
// ///
// /// The console will have inputs hidden while user types, and the returned
// /// value is a [`cu::ZString`](struct@crate::ZString)
// ///
// /// ```rust,no_run
// /// # use pistonite_cu as cu;
// /// let password = cu::prompt_password!("please enter your password")?;
// /// cu::info!("user entered: {password}");
// /// # cu::Ok(())
// /// ```
// #[cfg(feature = "prompt-password")]
// #[macro_export]
// macro_rules! prompt_password {
//     ($($fmt_args:tt)*) => {{
//         $crate::cli::__prompt(format_args!($($fmt_args)*), true)
//     }}
// }

// ///
// /// ```rust,no_run
// /// # use pistonite_cu as cu;
// /// // note that extra parenthesis is needed if the format args
// /// // are not inlined into the formatting literal
// /// let expected = "rust";
// /// let answer = cu::prompt_validate!(
// ///     ("what's your favorite programming language? please answer {}", expected),
// ///     |answer| {
// ///         if answer == expected {
// ///             return Ok(true);
// ///         }
// ///         if answer == "javascript" {
// ///             cu::bail!("that's not good");
// ///         }
// ///         cu::error!("try again");
// ///         Ok(false)
// ///     }
// /// )?;
// /// assert!(answer == expected);
// /// # cu::Ok(())
// /// ```
// ///
// /// The validation function can be a `FnMut` closure, which means
// /// it can double as a result parsing function if needed
// ///
// /// ```rust,no_run
// /// # use pistonite_cu as cu;
// /// let mut index: i32 = 0;
// /// cu::prompt_validate!(
// ///     "select a number between 0 and 5",
// ///     |answer| {
// ///         let number = match cu::parse::<i32>(answer) {
// ///             Err(e) => {
// ///                 cu::error!("{e}");
// ///                 cu::hint!("please ensure you are entering a number");
// ///                 return Ok(false);
// ///             }
// ///             Ok(x) => x
// ///         };
// ///         if number < 0 {
// ///             cu::error!("the number you entered is too small");
// ///             return Ok(false);
// ///         }
// ///         if number > 5 {
// ///             cu::error!("the number you entered is too big");
// ///             return Ok(false);
// ///         }
// ///         index = number;
// ///         Ok(true)
// ///     }
// /// )?;
// /// cu::info!("index is {index}");
// /// # cu::Ok(())
// /// ```
// ///
// /// For the password version, see [`prompt_password_validate`](crate::prompt_password_validate)
// ///
// #[cfg(feature = "prompt")]
// #[macro_export]
// macro_rules! prompt_validate {
//     ($l:literal, $validator:expr) => {{
//         $crate::cli::__prompt_with_validation(format_args!($l), false, $validator)
//         .map(|x| x.to_string())
//     }};
//     (($($fmt_args:tt)*), $validator:expr) => {{
//         $crate::cli::__prompt_with_validation(format_args!($($fmt_args)*), false, $validator)
//         .map(|x| x.to_string())
//     }}
// }

// /// Loop a password prompt until a validation function passes
// ///
// /// The validation function takes a `&mut String`,
// /// and returns `cu::Result<bool>`, where:
// /// - `Ok(true)` means the validation passed.
// /// - `Ok(false)` means the validation failed. The function can optionally
// ///   print some kind of error or hint message
// /// - `Err` means there is an error, the error will be propagated to the prompt call.
// ///
// /// ```rust,no_run
// /// # use pistonite_cu as cu;
// /// // note that extra parenthesis is needed if the format args
// /// // are not inlined into the formatting literal
// /// let password = cu::prompt_password_validate!(
// ///     "please enter a password between 8 and 16 charactres and only contain sensible characters",
// ///     |answer| {
// ///         if answer == "123456" {
// ///             cu::bail!("how can you do that, bye");
// ///         }
// ///         if answer.len() < 8 {
// ///             cu::error!("password is too short");
// ///             return Ok(false);
// ///         }
// ///         if answer.len() > 16 {
// ///             cu::error!("password is too long");
// ///             return Ok(false);
// ///         }
// ///         if let Err(e) = cu::password_chars_legal(answer) {
// ///             cu::error!("invalid password: {e}");
// ///             return Ok(false);
// ///         }
// ///         Ok(true)
// ///     }
// /// )?;
// /// cu::print!("{password}");
// /// # cu::Ok(())
// /// ```
// #[cfg(feature = "prompt-password")]
// #[macro_export]
// macro_rules! prompt_password_validate {
//     ($l:literal, $validator:expr) => {{
//         $crate::cli::__prompt_with_validation(format_args!($l), true, $validator)
//     }};
//     (($($fmt_args:tt)*), $validator:expr) => {{
//         $crate::cli::__prompt_with_validation(format_args!($($fmt_args)*), true, $validator)
//     }}
// }

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
