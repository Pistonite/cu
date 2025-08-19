use std::collections::BTreeSet;
use std::process::Stdio;
use std::sync::{Arc, LazyLock};

use regex::Regex;
use tokio::io::AsyncBufReadExt as _;
use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};

use crate::lv::Lv;
use crate::process::{Command, Preset, pio};
use crate::{BoxedFuture, ProgressBar};

/// Display progress of cargo task with a progress bar, and emitting
/// status messages and diagnostic messages using this crate's printing utilities.
///
/// `json` feature is required to enable parsing cargo's output messages.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// # fn main() -> cu::Result<()> {
/// cu::which("cargo")?.command()
///     .args(["build", "--release"])
///     .preset(cu::pio::cargo())
///     .spawn()?.0
///     .wait_nz()?;
/// # Ok(()) }
/// ```
///
/// # Behavior
/// - Added args: `--message-format json-diagnostic-rendered-ansi`
/// - All IO will be configured. You should avoid configuring IO by yourself
///   before or after applying this preset. This may become enforced in the future
///   through generics.
///
/// The progress is displayed on the progress bar, showing the current
/// crates being built in one line (similar to the build progress bar shown
/// by cargo).
///
/// # Message levels
/// Errors, warnings and status messages (like `Compiling foobar v0.1.0`)
/// can be configured with the [`error`](Cargo::error), [`warning`](Cargo::warning),
/// or [`other`](Cargo::other) functions that take a message level.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// cu::pio::cargo()
///     // configure message levels; levels shown here are the default
///     .error(cu::lv::E)
///     .warning(cu::lv::W)
///     .other(cu::lv::D);
/// ```
///
/// # Diagnostic hooks
/// To process diagnostic messages from cargo, you can provide a diagnostic hook,
/// which is a function `(is_warning: bool, message: &str) -> ()`.
/// If a diagnostic hook is provided, then the hook is responsible for displaying
/// the message. The `error` and `warning` levels will have no effect.
///
/// ```rust,no_run
/// # use pistonite_cu as cu;
/// use cu::pre::*;
///
/// cu::pio::cargo()
///     // configure message levels; levels shown here are the default
///     .on_diagnostic(|is_warning, message| {
///         // this implementation will be identical to the default behavior
///         if is_warning {
///             cu::warn!("{message}")
///         } else {
///             cu::error!("{message}")
///         }
///     });
/// ```
///
/// # Output
/// The handle to the progress bar is emitted to both the stdout and stderr slot.
/// Be sure to manually drop the handle to mark the progress as done if needed.
///
pub fn cargo() -> Cargo {
    Cargo {
        error_lv: Lv::Error,
        warning_lv: Lv::Warn,
        other_lv: Lv::Debug,
        diagnostic_hook: None,
    }
}
pub struct Cargo {
    error_lv: Lv,
    warning_lv: Lv,
    other_lv: Lv,
    diagnostic_hook: Option<DianogsticHook>,
}

impl Cargo {
    /// Set the level for printing error messages from cargo
    pub fn error(mut self, lv: Lv) -> Self {
        self.error_lv = lv;
        self
    }
    /// Set the level for printing warning messages from cargo
    pub fn warning(mut self, lv: Lv) -> Self {
        self.warning_lv = lv;
        self
    }
    /// Set the level for printing other messages from cargo
    pub fn other(mut self, lv: Lv) -> Self {
        self.other_lv = lv;
        self
    }
    /// Set a diagnostic hook, used to inspect compiler diagnostics from cargo
    ///
    /// The parameters are `(is_warning, message)`. The message is ansi-rendered.
    ///
    /// The hook should take care of printing the message
    pub fn on_diagnostic<F: Fn(bool, &str) + Send + 'static>(mut self, f: F) -> Self {
        self.diagnostic_hook = Some(Box::new(f));
        self
    }
}

impl Preset for Cargo {
    type Output = Command<Cargo, Cargo, pio::Null>;

    fn configure<O, E, I>(self, command: crate::Command<O, E, I>) -> Self::Output {
        command
            .args(["--message-format=json-diagnostic-rendered-ansi"])
            .stderr(Cargo {
                error_lv: self.error_lv,
                warning_lv: self.warning_lv,
                other_lv: self.other_lv,
                diagnostic_hook: None,
            })
            .stdout(self)
            .stdin_null()
    }
}
pub struct CargoTask {
    error_lv: Lv,
    warning_lv: Lv,
    other_lv: Lv,
    bar: Arc<ProgressBar>,
    out: ChildStdout,
    err: ChildStderr,
    diagnostic_hook: Option<DianogsticHook>,
}

impl pio::ChildOutConfig for Cargo {
    type Task = Option<CargoTask>;
    type __Null = super::__OCNonNull;
    fn configure_stdout(&mut self, command: &mut TokioCommand) {
        command.stdout(Stdio::piped());
    }
    fn configure_stderr(&mut self, command: &mut TokioCommand) {
        command.stderr(Stdio::piped());
    }
    fn take(
        self,
        child: &mut TokioChild,
        name: Option<&str>,
        is_out: bool,
    ) -> crate::Result<Self::Task> {
        // we need to take both out and err
        if !is_out {
            return Ok(None);
        }
        let stdout = super::take_child_stdout(child)?;
        let stderr = super::take_child_stderr(child)?;
        let bar = crate::progress_unbounded(name.unwrap_or("cargo"));
        Ok(Some(CargoTask {
            error_lv: self.error_lv,
            warning_lv: self.warning_lv,
            other_lv: self.other_lv,
            bar,
            out: stdout,
            err: stderr,
            diagnostic_hook: self.diagnostic_hook,
        }))
    }
}
impl pio::ChildOutTask for Option<CargoTask> {
    type Output = Option<Arc<ProgressBar>>;

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        match self {
            None => (None, None),
            Some(task) => {
                let bar = Arc::clone(&task.bar);
                (Some(Box::pin(task.main())), Some(bar))
            }
        }
    }
}

impl CargoTask {
    async fn main(self) {
        let read_out = tokio::io::BufReader::new(self.out);
        let mut out_lines = Some(read_out.lines());
        let read_err = tokio::io::BufReader::new(self.err);
        let mut err_lines = Some(read_err.lines());

        crate::progress!(&self.bar, (), "preparing");

        let mut state = PrintState {
            error_lv: self.error_lv,
            warning_lv: self.warning_lv,
            other_lv: self.other_lv,
            bar: self.bar,
            done_count: 0,
            in_progress: Default::default(),
            buf: Default::default(),
            diagnostic_hook: self.diagnostic_hook,
        };

        loop {
            let read_res = match (&mut out_lines, &mut err_lines) {
                (None, None) => break,
                (Some(out), None) => Ok(out.next_line().await),
                (None, Some(err)) => Err(err.next_line().await),
                (Some(out), Some(err)) => {
                    tokio::select! {
                        x = out.next_line() => Ok(x),
                        x = err.next_line() => Err(x)
                    }
                }
            };
            let line: Result<String, String> = match read_res {
                Ok(x) => match x {
                    Ok(Some(x)) => Ok(x),
                    _ => {
                        out_lines = None;
                        continue;
                    }
                },
                Err(x) => match x {
                    Ok(Some(x)) => Err(x),
                    _ => {
                        err_lines = None;
                        continue;
                    }
                },
            };
            match line {
                Ok(line) => state.handle_stdout(&line),
                Err(line) => state.handle_stderr(&line),
            }
        }
    }
}

struct PrintState {
    error_lv: Lv,
    warning_lv: Lv,
    other_lv: Lv,
    bar: Arc<ProgressBar>,
    done_count: usize,
    in_progress: BTreeSet<String>,
    buf: String,
    diagnostic_hook: Option<DianogsticHook>,
}

impl PrintState {
    fn handle_stdout(&mut self, line: &str) {
        // only handle json output from stdout
        if !line.starts_with('{') {
            crate::trace!("{line}");
            return;
        }

        let payload = match crate::json::parse::<Payload>(line) {
            Ok(x) => x,
            Err(e) => {
                crate::trace!("failed to parse cargo json output: {e:?}");
                return;
            }
        };
        match payload.reason {
            "compiler-artifact" => {
                let Some(target) = payload.target else {
                    return;
                };
                if target.name == "build-script-build" {
                    // skip processing build script builds
                    return;
                }
                self.done_count += 1;
                self.in_progress.remove(target.name);
                self.update_bar();
            }
            "compiler-message" => {
                let Some(message) = payload.message else {
                    return;
                };
                let Some(rendered) = message.rendered else {
                    return;
                };
                match message.level {
                    Some("warning") => match &self.diagnostic_hook {
                        None => {
                            crate::__priv::__print_with_level(
                                self.warning_lv,
                                format_args!("{rendered}"),
                            );
                        }
                        Some(hook) => hook(true, &rendered),
                    },
                    Some("error") => match &self.diagnostic_hook {
                        None => {
                            crate::__priv::__print_with_level(
                                self.error_lv,
                                format_args!("{rendered}"),
                            );
                        }
                        Some(hook) => hook(false, &rendered),
                    },
                    _ => {
                        crate::__priv::__print_with_level(
                            self.other_lv,
                            format_args!("{rendered}"),
                        );
                    }
                }
            }
            "build-finished" => match payload.success {
                Some(true) => {
                    crate::trace!("cargo build successful");
                }
                _ => {
                    crate::trace!("cargo build failed");
                }
            },
            "build-script-executed" => {}
            _ => {
                crate::trace!("unhandled cargo message reason: {}", payload.reason);
            }
        }
    }

    fn handle_stderr(&mut self, line: &str) {
        static STATUS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("^((\x1b[^m]*m)|\\s)*(Compiling|Checking)((\x1b[^m]*m)|\\s)*").unwrap()
        });
        crate::__priv::__print_with_level(self.other_lv, format_args!("{line}"));
        let Some(m) = STATUS_REGEX.find(line) else {
            return;
        };
        let line = &line[m.end()..].trim();
        // crate name can't have space (right?)
        let crate_name = match line.find(' ') {
            None => line,
            Some(i) => &line[..i],
        };
        self.in_progress.insert(crate_name.replace('-', "_"));
        self.update_bar();
    }

    fn update_bar(&mut self) {
        let count = self.done_count;
        self.buf.clear();
        let mut iter = self.in_progress.iter();
        if let Some(x) = iter.next() {
            self.buf.push_str(x);
            for c in iter {
                self.buf.push_str(", ");
                self.buf.push_str(c);
            }
            crate::progress!(&self.bar, (), "{count} done, compiling: {}", self.buf);
        } else if count != 0 {
            crate::progress!(&self.bar, (), "{count} done");
        }
    }
}

// (is_warning, message) -> Break = don't print, Continue = print original or overriden message
type DianogsticHook = Box<dyn Fn(bool, &str) + Send>;

#[derive(serde::Deserialize)]
struct Payload<'a> {
    reason: &'a str,
    target: Option<PayloadTarget<'a>>,
    message: Option<PayloadMessage<'a>>,
    success: Option<bool>,
}

#[derive(serde::Deserialize)]
struct PayloadTarget<'a> {
    name: &'a str,
}

#[derive(serde::Deserialize)]
struct PayloadMessage<'a> {
    level: Option<&'a str>,
    // for some reason, this can't be deserialize as borrowed
    rendered: Option<String>,
}
