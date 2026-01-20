use crate::cli::printer::PRINTER;
use crate::lv;
use crate::{Atomic, Context as _};

pub(crate) static PROMPT_LEVEL: Atomic<u8, lv::Prompt> =
    Atomic::new_u8(lv::Prompt::Interactive as u8);

type AnswerRecv = oneshot::Receiver<cu::Result<Option<cu::ZString>>>;

/// # Prompting
/// The `prompt` feature allows displaying prompts in the console to accept
/// user input. The prompts are thread-safe and synchronized with the printer.
/// When a prompt is active, outputs to the console will be buffered inside the printer.
/// Progress bars will also be paused.
///
/// The `--yes`, `--non-interactive`, and `--interactive` CLI flags are enabled
/// with the `prompt` feature, see [Command Line Interface](mod@crate::cli) for more information.
///
/// The [`PromptBuilder`](crate::cli::PromptBuilder) is the main struct used to configure the prompt.
/// The entry point to configuration are `cu::prompt()` and [`cu::yesno()`]. The default configuration
/// are as follows:
/// - `cu::prompt(message)`: No validation, returns `None` if user presses `Ctrl-C`
/// - `cu::yesno(messsage)`: Accepts case-insensitive `y`, `n`, `yes` or `no`, appends `" [y/n]"`
///   to the input message, and pressing `Ctrl-C` is the same as answering "no".
///
/// Call `.run()` to show the prompt and wait for the input:
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// match cu::prompt("please enter your name").run()? {
///     None => cu::info!("cancelled"), // pressed Ctrl-C
///     Some(name) => cu::info!("user entered: {name}"),
/// }
/// # cu::Ok(())
/// ```
///
/// # Macros
/// The [`cu::prompt!`] or the [`cu::yesno!`] macro are shorthands to show prompt for the most
/// common scenarios. They also accept format args:
/// - `cu::prompt!(format_args)`: No validation, **bails if user presses `Ctrl-C`**
/// - `cu::yesno!(format_args)`: Same behavior as `cu::yesno(message)`
///
/// These are designed to "just work" with `?` in 99% of the cases.
///
/// # Builder Methods
/// For advanced configurations, refer to the following builder methods
/// - [`password`](PromptBuilder::password): Hides the input when typing in console.
///   Also will force reading from terminal if stdin is not terminal.
/// - [`trim_trailing_whitespace`](PromptBuilder::trim_trailing_whitespace):
///   If disabled, trailing whitespaces are not trimmed. The output will still not contain
///   CRLF or LF at the end. (Default is enabled - will trim whitespace).
/// - [`validate_with`](PromptBuilder::validate_with): attaches a validation function. The prompt
///   will loop until the validation passed or an error occurs.
/// - [`if_cancel`](PromptBuilder::if_cancel): Set a default value to return if `Ctrl-C`
///   is pressed. This will make the prompt not return `Option`, but the inner value directly.
///   (Note the default value does not go through the validator)
/// - [`or_cancel`](PromptBuilder::or_cancel): Return a "canceled by user" error if `Ctrl-C` is pressed.
///   This also appends "(Ctrl-C to cancel)" to the prompt message.
///
/// Example:
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// let expected = "rust";
/// let answer: cu::ZString = cu::prompt(
///     format!("what's your favorite programming language? please answer {}", expected)
/// ).validate_with(
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
/// )
/// .if_cancel("javascript")
/// .run()?;
/// if answer == "javascript" {
///     cu::info!("hey, did you press Ctrl-C?");
/// } else {
///     cu::info!("your answer: {answer}");
/// }
/// # cu::Ok(())
/// ```
///
/// # Async
/// Use [`.co_run().await?`](PromptBuilder::co_run) instead of `.run()` in async
/// contexts to wait for the answer using the async runtime instead of blocking the current
/// thread.
///
/// When using the macros, add `async` to the beginning of the input
#[cfg_attr(feature = "coroutine", doc = "```rust,no_run")]
#[cfg_attr(not(feature = "coroutine"), doc = "```rust,ignore")]
/// # use pistonite_cu as cu;
/// # async fn main_() -> cu::Result<()> {
/// let answer = cu::prompt!(async "how are you today?")?;
/// cu::info!("you: {answer}");
/// # Ok(()) }
/// ```
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
#[inline(always)]
pub fn prompt(
    message: impl Into<String>,
) -> PromptBuilder<cu::ZString, Cancellable, impl FnMut(&mut String) -> cu::Result<bool>> {
    PromptBuilder::new(message)
}

/// Show a Yes/No prompt
///
/// *Note: Use the [`cu::yesno!`] macro if the default configuration is all you need*
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
/// if cu::yesno("do you want to continue?").run()? {
///     cu::info!("user picked yes");
/// }
/// # cu::Ok(())
/// ```
#[inline(always)]
pub fn yesno(
    message: impl Into<String>,
) -> PromptBuilder<bool, DefaultIfCancel, impl FnMut(&mut String) -> cu::Result<bool>> {
    let mut message = message.into();
    message.push_str(" [y/n]");
    PromptBuilder::<bool, _, _>::new(message).if_cancel(false)
}

// marker traits
#[doc(hidden)]
pub trait PromptCancelConfig {}
#[doc(hidden)]
pub struct Cancellable;
#[doc(hidden)]
pub struct DefaultIfCancel;
#[doc(hidden)]
pub struct BailIfCancel;
impl PromptCancelConfig for Cancellable {}
impl PromptCancelConfig for DefaultIfCancel {}
impl PromptCancelConfig for BailIfCancel {}

/// See [`Prompting`](fn@crate::prompt)
pub struct PromptBuilder<
    TOutput,
    TCancel: PromptCancelConfig,
    TValidate: FnMut(&mut String) -> cu::Result<bool>,
> {
    message: String,
    is_password: bool,
    trim_trailing_whitespace: bool,
    validator: TValidate,
    cancel_type: TCancel,
    cancel_value: Option<TOutput>,
}

impl<TOutput> PromptBuilder<TOutput, Cancellable, fn(&mut String) -> cu::Result<bool>> {
    #[inline(always)]
    fn new(message: impl Into<String>) -> Self {
        PromptBuilder {
            message: message.into(),
            is_password: false,
            trim_trailing_whitespace: true,
            validator: empty_validator,
            cancel_type: Cancellable,
            cancel_value: None,
        }
    }
}

impl<TCancel: PromptCancelConfig, TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<cu::ZString, TCancel, TValidate>
{
    /// Prompt for password. The input will be hidden.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() -> cu::Result<()> {
    /// match cu::prompt("enter your password:").password().run()? {
    ///     Some(password) => cu::info!("user entered: {password}"),
    ///     None => cu::error!("cancelled"),
    /// }
    /// # Ok(()) }
    /// ```
    #[inline(always)]
    pub fn password(mut self) -> Self {
        self.is_password = true;
        self
    }

    /// Configure whether to trim trailing whitespace from the input.
    ///
    /// By default, trailing whitespace is trimmed from user input. Set this to `false`
    /// to preserve trailing whitespace. Note that CRLF/LF line endings are always removed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() -> cu::Result<()> {
    /// let input = cu::prompt("enter text:").trim_trailing_whitespace(false).run()?;
    /// // trailing spaces will be preserved
    /// # Ok(()) }
    /// ```
    #[inline(always)]
    pub fn trim_trailing_whitespace(mut self, trim: bool) -> Self {
        self.trim_trailing_whitespace = trim;
        self
    }

    /// Add a validation function to the prompt.
    /// The prompt will be looped until a validation function passes.
    ///
    /// The validation function takes a `&mut String`,
    /// and returns `cu::Result<bool>`, where:
    /// - `Ok(true)` means the validation passed.
    /// - `Ok(false)` means the validation failed. The function can optionally
    ///   print some kind of error or hint message
    /// - `Err` means there is an error, the error will be propagated to the prompt call.
    ///
    /// The validation function can be a `FnMut` closure, which means
    /// it can double as a result parsing function if needed
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// let mut index: i32 = 0;
    /// cu::prompt("select a number between 0 and 5")
    ///     .validate_with(
    ///         |answer| {
    ///             let number = match cu::parse::<i32>(answer) {
    ///                 Err(e) => {
    ///                     cu::error!("{e}");
    ///                     cu::hint!("please ensure you are entering a number");
    ///                     return Ok(false);
    ///                 }
    ///                 Ok(x) => x
    ///             };
    ///             if number < 0 {
    ///                 cu::error!("the number you entered is too small");
    ///                 return Ok(false);
    ///             }
    ///             if number > 5 {
    ///                 cu::error!("the number you entered is too big");
    ///                 return Ok(false);
    ///             }
    ///             index = number;
    ///             Ok(true)
    ///         }
    ///     ).run()?;
    /// cu::info!("index is {index:?}");
    /// # cu::Ok(())
    /// ```
    #[inline(always)]
    pub fn validate_with<F>(self, validator: F) -> PromptBuilder<cu::ZString, TCancel, F>
    where
        F: FnMut(&mut String) -> cu::Result<bool>,
    {
        PromptBuilder {
            message: self.message,
            is_password: self.is_password,
            trim_trailing_whitespace: self.trim_trailing_whitespace,
            validator,
            cancel_type: self.cancel_type,
            cancel_value: self.cancel_value,
        }
    }
}
impl<TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<cu::ZString, Cancellable, TValidate>
{
    /// Convert to a yes/no confirmation prompt. The prompt will accept
    /// "yes", "y", "no", or "n" (case-insensitive) as valid input.
    ///
    /// Unlike [`cu::yesno`](fn@crate::yesno), `.prompt().yesno()`
    /// will let you detect cancellations by pressing `Ctrl-C`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() -> cu::Result<()> {
    /// match cu::prompt("delete all files?").yesno().run()? {
    ///     None => cu::error!("cancelled"),
    ///     Some(true) => cu::info!("all files are deleted!"),
    ///     Some(false) => cu::error!("user explicitly answered no"),
    /// }
    /// # Ok(()) }
    /// ```
    #[inline(always)]
    pub fn yesno(mut self) -> PromptBuilder<bool, Cancellable, TValidate> {
        self.message.push_str(" [y/n]");
        PromptBuilder {
            message: self.message,
            is_password: self.is_password,
            trim_trailing_whitespace: self.trim_trailing_whitespace,
            validator: self.validator,
            cancel_type: Cancellable,
            cancel_value: None,
        }
    }

    /// Set a default value to return if the user cancels the prompt (e.g., Ctrl+C).
    ///
    /// Without `if_cancel`, the prompt returns `Option<ZString>` where `None`
    /// indicates cancellation. With `if_cancel`, the prompt returns `ZString`
    /// directly, using the provided default value on cancellation.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() -> cu::Result<()> {
    /// let name: cu::ZString = cu::prompt("enter your name:")
    ///     .if_cancel("anonymous")
    ///     .run()?;
    /// cu::info!("Hello, {name}!");
    /// # Ok(()) }
    /// ```
    ///
    /// This has the same effect as calling `unwrap_or()` on the resulting
    /// `Option<ZString>` - Just that the prompt creator has control over
    /// the outcome of the prompt, rather than the prompt consumer accessing the outcome.
    #[inline(always)]
    pub fn if_cancel(
        self,
        default: impl Into<String>,
    ) -> PromptBuilder<cu::ZString, DefaultIfCancel, TValidate> {
        PromptBuilder {
            message: self.message,
            is_password: self.is_password,
            trim_trailing_whitespace: self.trim_trailing_whitespace,
            validator: self.validator,
            cancel_type: DefaultIfCancel,
            cancel_value: Some(default.into().into()),
        }
    }

    /// Return an error if the user cancels the prompt (e.g., Ctrl+C).
    ///
    /// This appends "(Ctrl-C to cancel)" to the prompt message and returns
    /// an error with message "operation cancelled by user" if the user presses Ctrl+C.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() -> cu::Result<()> {
    /// let name: cu::ZString = cu::prompt("enter your name:").or_cancel().run()?;
    /// cu::info!("Hello, {name}!");
    /// # Ok(()) }
    /// ```
    #[inline(always)]
    pub fn or_cancel(mut self) -> PromptBuilder<cu::ZString, BailIfCancel, TValidate> {
        self.message.push_str(" (Ctrl-C to cancel)");
        PromptBuilder {
            message: self.message,
            is_password: self.is_password,
            trim_trailing_whitespace: self.trim_trailing_whitespace,
            validator: self.validator,
            cancel_type: BailIfCancel,
            cancel_value: None,
        }
    }

    /// Show the prompt and block the current thread to wait for input.
    ///
    /// If the user presses `Ctrl-C`, `None` will be returned.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`co_run().await?`](`Self::co_run`) instead.
    pub fn run(self) -> cu::Result<Option<cu::ZString>> {
        check_prompt_level(false)?;
        run_prompt_loop(
            self.message,
            self.is_password,
            self.trim_trailing_whitespace,
            self.validator,
        )
    }

    /// Show the prompt and use the async runtime to wait for input.
    ///
    /// If the user presses `Ctrl-C`, `None` will be returned.
    pub async fn co_run(self) -> cu::Result<Option<cu::ZString>> {
        check_prompt_level(false)?;
        co_run_prompt_loop(
            self.message,
            self.is_password,
            self.trim_trailing_whitespace,
            self.validator,
        )
        .await
    }
}
impl<TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<cu::ZString, DefaultIfCancel, TValidate>
{
    /// Show the prompt and block the current thread to wait for input.
    ///
    /// If the user presses `Ctrl-C`, the default value set by [`if_cancel`](Self::if_cancel)
    /// will be returned.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`co_run().await?`](`Self::co_run`) instead.
    pub fn run(self) -> cu::Result<cu::ZString> {
        check_prompt_level(false)?;
        // unwrap: safety from builder
        let result = run_prompt_loop(
            self.message,
            self.is_password,
            self.trim_trailing_whitespace,
            self.validator,
        )?
        .unwrap_or(self.cancel_value.unwrap());
        Ok(result)
    }

    /// Show the prompt and use the async runtime to wait for input.
    ///
    /// If the user presses `Ctrl-C`, the default value set by [`if_cancel`](Self::if_cancel)
    /// will be returned.
    pub async fn co_run(self) -> cu::Result<cu::ZString> {
        check_prompt_level(false)?;
        // unwrap: safety from builder
        let result = co_run_prompt_loop(
            self.message,
            self.is_password,
            self.trim_trailing_whitespace,
            self.validator,
        )
        .await?
        .unwrap_or(self.cancel_value.unwrap());
        Ok(result)
    }
}
impl<TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<cu::ZString, BailIfCancel, TValidate>
{
    /// Show the prompt and block the current thread to wait for input.
    ///
    /// If the user presses `Ctrl-C`, an error with message "operation cancelled by user"
    /// will be returned.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`co_run().await?`](`Self::co_run`) instead.
    pub fn run(self) -> cu::Result<cu::ZString> {
        check_prompt_level(false)?;
        match run_prompt_loop(
            self.message,
            self.is_password,
            self.trim_trailing_whitespace,
            self.validator,
        )? {
            Some(result) => Ok(result),
            None => crate::bail!("operation cancelled by user"),
        }
    }

    /// Show the prompt and use the async runtime to wait for input.
    ///
    /// If the user presses `Ctrl-C`, an error with message "operation cancelled by user"
    /// will be returned.
    pub async fn co_run(self) -> cu::Result<cu::ZString> {
        check_prompt_level(false)?;
        match co_run_prompt_loop(
            self.message,
            self.is_password,
            self.trim_trailing_whitespace,
            self.validator,
        )
        .await?
        {
            Some(result) => Ok(result),
            None => crate::bail!("operation cancelled by user"),
        }
    }
}
impl<TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<bool, Cancellable, TValidate>
{
    /// Set a default value to return if the user cancels the prompt (e.g., Ctrl+C).
    ///
    /// Without `if_cancel`, the prompt returns `Option<bool>` where `None`
    /// indicates cancellation. With `if_cancel`, the prompt returns `bool`
    /// directly, using the provided default value on cancellation.
    ///
    /// Note that [`cu::yesno`](fn@crate::yesno) already calls `.if_cancel(false)`
    /// internally.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() -> cu::Result<()> {
    /// if cu::prompt("proceed?").yesno().if_cancel(false).run()? {
    ///     cu::info!("user answered yes!");
    /// }
    /// # Ok(()) }
    /// ```
    ///
    /// This has the same effect as calling `unwrap_or()` on the resulting
    /// `Option<bool>` - Just that the prompt creator has control over
    /// the outcome of the prompt, rather than the prompt consumer accessing the outcome.
    #[inline(always)]
    pub fn if_cancel(self, default: bool) -> PromptBuilder<bool, DefaultIfCancel, TValidate> {
        PromptBuilder {
            message: self.message,
            is_password: self.is_password,
            trim_trailing_whitespace: self.trim_trailing_whitespace,
            validator: self.validator,
            cancel_type: DefaultIfCancel,
            cancel_value: Some(default),
        }
    }

    /// Return an error if the user cancels the prompt (e.g., Ctrl+C).
    ///
    /// This appends "(Ctrl-C to cancel)" to the prompt message and returns
    /// an error with message "operation cancelled by user" if the user presses Ctrl+C.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pistonite_cu as cu;
    /// # fn main() {
    /// match cu::prompt("proceed?").yesno().or_cancel().run() {
    ///     // user answered yes
    ///     Ok(true) => cu::info!("proceeding..."),
    ///     // user answered no
    ///     Ok(false) => cu::warn!("user declined"),
    ///     // user pressed Ctrl-C, or prompt not allowed
    ///     Err(e) => cu::error!("{e}"),
    /// }
    /// # }
    /// ```
    #[inline(always)]
    pub fn or_cancel(mut self) -> PromptBuilder<bool, BailIfCancel, TValidate> {
        self.message.push_str(" (Ctrl-C to cancel)");
        PromptBuilder {
            message: self.message,
            is_password: self.is_password,
            trim_trailing_whitespace: self.trim_trailing_whitespace,
            validator: self.validator,
            cancel_type: BailIfCancel,
            cancel_value: None,
        }
    }

    /// Show the prompt and block the current thread to wait for input.
    ///
    /// If the user presses `Ctrl-C`, `None` will be returned.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`co_run().await?`](`Self::co_run`) instead.
    pub fn run(self) -> cu::Result<Option<bool>> {
        if check_prompt_level(true)? {
            return Ok(Some(true));
        }
        run_yesno_loop(self.message, self.is_password)
    }

    /// Show the prompt and use the async runtime to wait for input.
    ///
    /// If the user presses `Ctrl-C`, `None` will be returned.
    pub async fn co_run(self) -> cu::Result<Option<bool>> {
        if check_prompt_level(true)? {
            return Ok(Some(true));
        }
        co_run_yesno_loop(self.message, self.is_password).await
    }
}
impl<TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<bool, BailIfCancel, TValidate>
{
    /// Show the prompt and block the current thread to wait for input.
    ///
    /// If the user presses `Ctrl-C`, an error with message "operation cancelled by user"
    /// will be returned.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`co_run().await?`](`Self::co_run`) instead.
    pub fn run(self) -> cu::Result<bool> {
        if check_prompt_level(true)? {
            return Ok(true);
        }
        match run_yesno_loop(self.message, self.is_password)? {
            Some(result) => Ok(result),
            None => crate::bail!("operation cancelled by user"),
        }
    }

    /// Show the prompt and use the async runtime to wait for input.
    ///
    /// If the user presses `Ctrl-C`, an error with message "operation cancelled by user"
    /// will be returned.
    pub async fn co_run(self) -> cu::Result<bool> {
        if check_prompt_level(true)? {
            return Ok(true);
        }
        match co_run_yesno_loop(self.message, self.is_password).await? {
            Some(result) => Ok(result),
            None => crate::bail!("operation cancelled by user"),
        }
    }
}
impl<TValidate: FnMut(&mut String) -> cu::Result<bool>>
    PromptBuilder<bool, DefaultIfCancel, TValidate>
{
    /// Show the prompt and block the current thread to wait for input.
    ///
    /// If the user presses `Ctrl-C`, the default value set by [`if_cancel`](Self::if_cancel)
    /// will be returned.
    ///
    /// # Blocking
    /// **Do not use this in an async context**, since it will block the runtime,
    /// and will panic if the thread is currently driving IO.
    /// Use [`co_run().await?`](`Self::co_run`) instead.
    pub fn run(self) -> cu::Result<bool> {
        if check_prompt_level(true)? {
            return Ok(true);
        }
        // unwrap: safety from builder
        Ok(run_yesno_loop(self.message, self.is_password)?.unwrap_or(self.cancel_value.unwrap()))
    }

    /// Show the prompt and use the async runtime to wait for input.
    ///
    /// If the user presses `Ctrl-C`, the default value set by [`if_cancel`](Self::if_cancel)
    /// will be returned.
    pub async fn co_run(self) -> cu::Result<bool> {
        if check_prompt_level(true)? {
            return Ok(true);
        }
        // unwrap: safety from builder
        Ok(co_run_yesno_loop(self.message, self.is_password)
            .await?
            .unwrap_or(self.cancel_value.unwrap()))
    }
}

fn run_yesno_loop(message: String, is_password: bool) -> cu::Result<Option<bool>> {
    let mut answer = false;
    let _ = cu::some!(run_prompt_loop(
        message,
        is_password,
        false, // we trim both ends when parsing
        |x| {
            match parse_yesno(x) {
                Some(x) => {
                    answer = x;
                    Ok(true)
                }
                None => {
                    cu::hint!("please enter yes or no");
                    Ok(false)
                }
            }
        }
    )?);
    Ok(Some(answer))
}
async fn co_run_yesno_loop(message: String, is_password: bool) -> cu::Result<Option<bool>> {
    let mut answer = false;
    let _ = cu::some!(
        co_run_prompt_loop(
            message,
            is_password,
            false, // we trim both ends when parsing
            |x| {
                match parse_yesno(x) {
                    Some(x) => {
                        answer = x;
                        Ok(true)
                    }
                    None => {
                        cu::hint!("please enter yes or no");
                        Ok(false)
                    }
                }
            }
        )
        .await?
    );
    Ok(Some(answer))
}
#[inline]
fn parse_yesno(x: &mut str) -> Option<bool> {
    x.make_ascii_lowercase();
    match x.trim() {
        "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        _ => None,
    }
}

#[inline(always)]
fn run_prompt_loop<F: FnMut(&mut String) -> cu::Result<bool>>(
    message: String,
    is_password: bool,
    trim_trailing_whitespace: bool,
    mut validator: F,
) -> cu::Result<Option<cu::ZString>> {
    loop {
        let result = do_show_prompt(&message, is_password)?;
        let result = cu::check!(result.recv(), "failed to receive answer to prompt")?;
        let result = cu::check!(result, "an error occured while processing a prompt")?;
        let mut result = cu::some!(result);
        if trim_trailing_whitespace {
            let len = result.trim_end().len();
            result.truncate(len);
        }
        if validator(&mut result)? {
            return Ok(Some(result));
        }
    }
}
#[inline(always)]
async fn co_run_prompt_loop<F: FnMut(&mut String) -> cu::Result<bool>>(
    message: String,
    is_password: bool,
    trim_trailing_whitespace: bool,
    mut validator: F,
) -> cu::Result<Option<cu::ZString>> {
    loop {
        let result = do_show_prompt(&message, is_password)?;
        let result = cu::check!(result.await, "failed to receive answer to prompt")?;
        let result = cu::check!(result, "an error occured while processing a prompt")?;
        let mut result = cu::some!(result);
        if trim_trailing_whitespace {
            let len = result.trim_end().len();
            result.truncate(len);
        }
        if validator(&mut result)? {
            return Ok(Some(result));
        }
    }
}

fn do_show_prompt(message: &str, is_password: bool) -> cu::Result<AnswerRecv> {
    if let Ok(mut printer) = PRINTER.lock()
        && let Some(printer) = printer.as_mut()
    {
        Ok(printer.show_prompt(message, is_password))
    } else {
        crate::bail!("prompt failed: failed to lock global printer");
    }
}

// Ok(true) -> answer Yes
// Ok(false) -> prompt
// Err -> prompt not allowed
fn check_prompt_level(is_yesno: bool) -> crate::Result<bool> {
    if is_yesno {
        match PROMPT_LEVEL.get() {
            // do not even show the prompt if --yes
            lv::Prompt::YesOrInteractive | lv::Prompt::YesOrBlock => return Ok(true),
            lv::Prompt::Interactive => return Ok(false),
            lv::Prompt::Block => {}
        }
    } else {
        if !matches!(
            PROMPT_LEVEL.get(),
            lv::Prompt::YesOrBlock | lv::Prompt::Block
        ) {
            return Ok(false);
        }
    }
    crate::bail!("prompt not allowed with --non-interactive");
}

#[inline(always)]
fn empty_validator(_: &mut String) -> cu::Result<bool> {
    Ok(true)
}
