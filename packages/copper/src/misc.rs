use std::any::Any;

/// Try to get info from a panic payload
pub fn best_effort_panic_info<'a>(payload: &'a Box<dyn Any + Send + 'static>) -> &'a str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        crate::debug!(
            "encountered unknown panic info with type id: {:?}",
            (**payload).type_id()
        );
        "unknown panic info"
    }
}

/// Like `unimplemented!` in std library, but log a message
/// and return an error instead of panicking
#[macro_export]
macro_rules! noimpl {
    () => {
        $crate::bailand!(error!("not implemented"))
    };
    ($($args:tt)*) => {{
        let msg = format!("not implemented: {}", format_args!($(args)*));
        $crate::bailand!(error!("{msg}"))
    }}
}

/// Check a `Result`, unwrapping the value or giving it a context
/// and return the error.
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
///     let foo = cu::check!(some_fallable_func(), "failed: {input}")?;
///     assert_eq!(foo, "foo");
///     // also log the error as we return the Err
///     let foo = cu::check!(some_fallable_func(), error!("failed: {input}"))?;
///     assert_eq!(foo, "foo");
///
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! check {
    ($result:expr, $mac:ident !( $($args:tt)* )) => {{
        { $result }.with_context(|| $crate::fmtand!($mac!($($args)*)))
    }};
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
    ($result:expr, $mac:ident !( $($args:tt)* )) => {{
        return Err($result).context($crate::fmtand!($mac!($($args)*)));
    }};
    ($result:expr, $($args:tt)*) => {{
        return Err($result).context(format!($($args)*));
    }};
}

/// Format and invoke a print macro
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// let x = cu::fmtand!(error!("found {} errors", 3));
/// assert_eq!(x, "found 3 errors");
/// ```
#[macro_export]
macro_rules! fmtand {
    ($mac:ident !( $($fmt_args:tt)* )) => {{
        let s = format!($($fmt_args)*);
        $crate::$mac!("{s}");
        s
    }}
}
/// Invoke a print macro, then bail with the same message
///
/// # Example
/// ```rust
/// # use pistonite_cu as cu;
/// # fn main() {
/// fn fn_1() -> cu::Result<()> {
///     cu::bailand!(error!("found {} errors", 3));
/// }
/// fn fn_2() -> cu::Result<()> {
///     cu::bailand!(warn!("warning!"));
/// }
/// assert!(fn_1().is_err()); // will also log error "found 3 errors"
/// assert!(fn_2().is_err()); // will also log warning "warning!"
/// # }
/// ```
#[macro_export]
macro_rules! bailand {
    ($mac:ident !( $($fmt_args:tt)* )) => {{
        let s = format!($($fmt_args)*);
        $crate::$mac!("{s}");
        $crate::bail!(s);
    }}
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
