
use super::Command;


/// Add arguments to the command
pub trait Config {
    fn configure(self, command: &mut Command);
}

pub struct __ConfigFn<F>(F) where F: FnOnce(&mut Command);
impl<F: FnOnce(&mut Command)> Config for __ConfigFn<F> {
    #[inline(always)]
    fn configure(self, command: &mut Command) {
        self.0(command)
    }
}

/// Create a config to add multiple args of different types when building
/// a subprocess.
///
/// # Example
/// ```rust,no_run
/// let path = Path::new("foo");
/// cu::bin::which("ls").unwrap()
///    .command()
///    .add(cu::args![path, "-a"]);
/// ```
#[macro_export]
macro_rules! args {
    ($($arg:expr),* $(,)?) => {
        $crate::__priv::__ConfigFn(|c| {
            $( c.arg($arg); )*
        })
    };
}

/// Create a config to add multiple environments of different types when building
/// a subprocess.
///
/// # Example
/// ```rust,no_run
/// let path = Path::new("bizbar");
/// cu::bin::which("foo").unwrap()
///    .command()
///    .add(cu::envs!{
///         "BAR" => "true",
///         "BIZ" => path
///    })
/// ```
#[macro_export]
macro_rules! envs {
    ($($k:expr => $v:expr),* $(,)?) => {
        $crate::__priv::__ConfigFn(|c| {
            $( c.env($k, $v); )*
        })
    };
}
