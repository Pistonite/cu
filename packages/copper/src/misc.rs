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
#[macro_export]
macro_rules! check {
    ($result:expr, $msg:literal) => {
        { $result }.context($msg)?
    };
    ($result:expr, $($args:tt)*) => {{
        { $result }.with_context(|| format!($($args)*))?
    }};
    ($result:expr, $mac:ident ! $($args:tt)*) => {{
        { $result }.with_context(|| $crate::fmtand!($mac!($($args)*)))?
    }};
}

/// Format and invoke a print macro
///
/// # Example
/// ```rust
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
