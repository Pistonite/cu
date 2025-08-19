use std::collections::BTreeSet;
use std::process::Stdio;
use std::sync::{Arc, LazyLock};

use regex::Regex;
use tokio::process::{Child as TokioChild, ChildStderr, ChildStdout, Command as TokioCommand};
use tokio::io::AsyncBufReadExt as _;

use crate::lv::Lv;
use crate::{BoxedFuture, ProgressBar};
use crate::process::{Command, Preset, pio};


pub fn cargo() -> Cargo {
    Cargo {
        error_lv: Lv::Error,
        warning_lv: Lv::Warn,
        other_lv: Lv::Debug,
    }
}
#[derive(Clone, Copy)]
pub struct Cargo {
    error_lv: Lv,
    warning_lv: Lv,
    other_lv: Lv,
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
}

impl Preset for Cargo {
    type Output = Command<Cargo, Cargo, pio::Null>;

    fn configure< O, E, I, >(self, command: crate::Command<O, E, I>) -> Self::Output {
        command.args([
            "--message-format=json-diagnostic-rendered-ansi"
        ])
        .stdoe(self)
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
}

impl pio::ChildOutConfig for Cargo {
    type Task = Option<CargoTask>;
    type __Null = super::__OCNull;
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
        }))
    }
}
impl pio::ChildOutTask for Option<CargoTask> {
    type Output = ();

    fn run(self) -> (Option<BoxedFuture<()>>, Self::Output) {
        match self {
            None => (None, ()),
            Some(task) => (Some(Box::pin(task.main())), ())
        }
    }
}

impl CargoTask {
    async fn main(self) {
        let read_out = tokio::io::BufReader::new(self.out);
        let mut out_lines = Some(read_out.lines());
        let read_err = tokio::io::BufReader::new(self.err);
        let mut err_lines = Some(read_err.lines());

        let mut state = PrintState { 
            error_lv: self.error_lv, 
            warning_lv: self.warning_lv, 
            other_lv: self.other_lv, 
            bar: self.bar, 
            done_count: 0, 
            in_progress: Default::default(), 
            buf: Default::default()
        };

        loop {
            let read_res = match (&mut out_lines, &mut err_lines) {
                (None, None) => break,
                (Some(out), None) => {
                    Ok(out.next_line().await)
                }
                (None, Some(err)) => {
                    Err(err.next_line().await)
                }
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
                }
                Err(x) => match x {
                    Ok(Some(x)) => Err(x),
                    _ => {
                        err_lines = None;
                        continue;
                    }
                }
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
                    Some("warning") => {
                        crate::__priv::__print_with_level(self.warning_lv, format_args!("{rendered}"));
                    }
                    Some("error") => {
                        crate::__priv::__print_with_level(self.error_lv, format_args!("{rendered}"));
                    }
                    _ => {
                        crate::__priv::__print_with_level(self.other_lv, format_args!("{rendered}"));
                    }
                }
            }
            "build-finished" => {
                match payload.success {
                    Some(true) => {
                        crate::trace!("cargo build successful");
                    }
                    _ => {
                        crate::trace!("cargo build failed");
                    }
                }
            }
            "build-script-executed" => {}
            _ => {
                crate::trace!("unhandled cargo message reason: {}", payload.reason);
            }
        }
    }

    fn handle_stderr(&mut self, line: &str) {
        static STATUS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("^((\x1b[^m]*m)|\\s)*Compiling((\x1b[^m]*m)|\\s)*")
                .unwrap()
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
            crate::progress!(&self.bar, (), "{count} done, compiling {}", self.buf);
        } else {
            crate::progress!(&self.bar, (), "{count} done");
        }
    }
}


#[derive(serde::Deserialize)]
struct Payload<'a> {
    reason: &'a str,
    target: Option<PayloadTarget<'a>>,
    message: Option<PayloadMessage<'a>>,
    success: Option<bool>
}

#[derive(serde::Deserialize)]
struct PayloadTarget<'a> {
    name: &'a str,
}

#[derive(serde::Deserialize)]
struct PayloadMessage<'a> {
    level: Option<&'a str>,
    // for some reason, this can't be deserialize as borrowed
    rendered: Option<String>
}
