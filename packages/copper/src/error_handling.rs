pub use anyhow::{Context, Error, Ok, Result, anyhow as fmterr, bail};

/// # Error Handling
/// *Does not require any feature flag. Please make sure to sponsor [David Tolnay](https://github.com/dtolnay) if you depend heavily on his work
/// on the Rust ecosystem.*
///
/// Most of the error handling stuff is re-exported from [`anyhow`](https://docs.rs/anyhow),
/// which is a crate that makes tracing and formatting error messages SUPER easy.
/// This is the easiest way to quickly write debuggable program without structured error.
/// Structured error types would be more useful if you are making a library though.
///
/// The traits required for error handling are included in the prelude import
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
/// ```
///
/// Here are the most commonly used `anyhow` re-exports
/// - `anyhow::Result` is `cu::Result`
/// - `anyhow::bail!`  is `cu::bail!`
/// - `anyhow::Ok`     is `cu::Ok`
///
/// Here are custom utilities from `cu` that integrates with `anyhow`
/// - `cu::check!` wraps `.with_context()`
///    ```rust
///    # use pistonite_cu as cu;
///    use cu::pre::*;
///
///    fn some_fallable_func() -> cu::Result<String> {
///        Ok("foo".to_string())
///    }
///    fn main() -> cu::Result<()> {
///        // this input is just to show the formatting
///        let input: i32 = 42;
///
///        let foo = cu::check!(some_fallable_func(), "failed: {input}")?;
///        // with anyhow, this would be:
///        // let foo = some_fallable_func().with_context(|| format!("failed: {input}"))?;
///        // -- much longer!
///        assert_eq!(foo, "foo");
///        Ok(())
///    }
///    ```
/// - [`cu::rethrow!`](macro@crate::rethrow) is similar to `bail!`, but works with an `Error` instance at hand
/// - [`cu::unimplemented!`](macro@crate::unimplemented)
///   and [`cu::unreachable!`](macro@crate::unreachable)
///   that are similar to the std macros, but instead of `panic!`, they will `bail!`
/// - [`cu::ensure`](macro@crate::ensure) is unlike `anyhow::ensure`, that
///   it evaluates to a `Result<()>` instead of generates a return.
///   It also does not automatically generate debug information.
///
/// Here are other `anyhow` re-exports that are less commonly used
/// - `anyhow::anyhow` is `cu::fmterr`
///
/// Finally, if you do need to panic, [`cu::panicand`](macro@crate::panicand)
/// allows you to also log the same message so you can debug it easier.
///
#[macro_export]
macro_rules! check {
    ($result:expr, $($args:tt)*) => {{
        { $result }.with_context(|| format!($($args)*))
    }};
}

/// Rethrow an `Err`, optionally with additional context
///
/// This is useful if the error path requires additional handling
///
/// Prelude import is required to bring in the Context trait.
///
/// ```rust
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// fn some_fallable_func() -> cu::Result<String> {
///     Ok("foo".to_string())
/// }
///
/// fn main() -> cu::Result<()> {
///     // this input is just to show the formatting
///     let input: i32 = 42;
///
///     let foo = match some_fallable_func() {
///         Ok(x) => x,
///         Err(e) => {
///             // supposed some additional handling is needed,
///             // like setting some error state...
///
///             cu::rethrow!(e, "failed: {input}");
///         }
///     };
///
///     assert_eq!(foo, "foo");
///
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! rethrow {
    ($result:expr) => {
        return Err($result);
    };
    ($result:expr, $($args:tt)*) => {{
        return Err($result).context(format!($($args)*));
    }};
}

/// Like `unimplemented!` in std library, but log a message
/// and return an error instead of panicking
#[macro_export]
macro_rules! unimplemented {
    () => {
        $crate::trace!("unexpected: not implemented reached");
        return $crate::Error::msg("not implemented");
    };
    ($($args:tt)*) => {{
        let msg = format!("{}", format_args!($(args)*));
        $crate::trace!("unexpected: not implemented reached: {msg}");
        $crate::bail!("not implemented: {msg}")
    }}
}

/// Like `unreachable!` in std library, but log a message
/// and return an error instead of panicking reached.
/// This might be less performant in release builds
#[macro_export]
macro_rules! unreachable {
    () => {
        $crate::trace!("unexpected: entered unreachable code");
        return $crate::Error::msg("unreachable");
    };
    ($($args:tt)*) => {{
        let msg = format!("{}", format_args!($(args)*));
        $crate::trace!("unexpected: entered unreachable code: {msg}");
        $crate::bail!("unreachable: {msg}")
    }}
}

/// Check if an expression is `true`
///
/// Unlike `anyhow::ensure`, if the condition fail, this will generate an `Error`
/// instead of returning an error directly, so you need to add a `?`.
/// It also always include the expression stringified in the debug info.
/// However, it does not automatically parse the input and generate debug
/// info message based on that (unlike `anyhow`)
#[macro_export]
macro_rules! ensure {
    ($result:expr) => {{
        if !bool::from($result) {
            Err($crate::fmterr!("condition failed: `{}`", stringify!($result)))
        } else {
            Ok(())
        }
    }};
    ($result:expr, $($args:tt)*) => {{
        if !bool::from($result) {
            Err($crate::fmterr!("condition failed: `{}`: {}", stringify!($result), format_args!($($args)*)))
        } else {
            Ok(())
        }
    }};
}

/// Invoke a print macro, then panic with the same message
///
/// # Example
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// cu::panicand!(error!("found {} errors", 3));
/// ```
#[macro_export]
macro_rules! panicand {
    ($mac:ident !( $($fmt_args:tt)* )) => {{
        let s = format!($($fmt_args)*);
        $crate::$mac!("{s}");
        panic!("{s}");
    }}
}
