use std::collections::VecDeque;
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use super::{FormatBuffer, ProgressBar, ansi};

use crate::{ZeroWhenDropString, lv};

/// Print something
///
/// This is similar to `info`, but unlike info, this message will still log with `-q`.
#[macro_export]
macro_rules! print {
    ($($fmt_args:tt)*) => {{
        $crate::__priv::__print_with_level($crate::lv::P, format_args!($($fmt_args)*));
    }}
}
/// Logs a hint message
#[macro_export]
macro_rules! hint {
    ($($fmt_args:tt)*) => {{
        $crate::__priv::__print_with_level($crate::lv::H, format_args!($($fmt_args)*));
    }}
}

/// Internal print function for macros
pub fn __print_with_level(lv: lv::Lv, message: std::fmt::Arguments<'_>) {
    if !lv.can_print(lv::PRINT_LEVEL.get()) {
        return;
    }
    let message = format!("{message}");
    if let Ok(mut printer) = PRINTER.lock() {
        printer.print_message(lv, &message);
    }
}

pub(crate) static PRINTER: LazyLock<Mutex<Printer>> =
    LazyLock::new(|| Mutex::new(Printer::default()));

/// Global printer state
pub(crate) struct Printer {
    /// Handle to stdout
    stdout: std::io::Stdout,
    /// Handle to stderr
    stderr: std::io::Stderr,
    /// Color codes
    colors: ansi::Colors,
    /// Control codes
    controls: ansi::Controls,

    // printing
    /// Handle for the printing task, None means
    /// either no printing task is running, or, the printing
    /// task is terminating
    print_task: PrintThread,
    bar_target: Option<Target>,
    bars: Vec<Weak<ProgressBar>>,

    prompt_task: PrintThread,
    pending_prompts: VecDeque<PromptTask>,

    /// Buffer for automatically do certain formatting
    format_buffer: FormatBuffer,
    /// Place to buffer prints while printing is blocked
    buffered: String,
}

struct PromptTask {
    send: oneshot::Sender<std::io::Result<ZeroWhenDropString>>,
    prompt: String,
    #[cfg(feature = "prompt-password")]
    is_password: bool,
}

impl Default for Printer {
    fn default() -> Self {
        use std::io::IsTerminal as _;
        let stdout = std::io::stdout();
        let stderr = std::io::stderr();
        let is_terminal = stdout.is_terminal();
        let bar_target = if is_terminal {
            Some(Target::Stdout)
        } else if stderr.is_terminal() {
            Some(Target::Stderr)
        } else {
            None
        };
        let colors = ansi::colors(is_terminal);
        let controls = ansi::controls(is_terminal);

        Self {
            stdout,
            stderr,
            colors,
            controls,
            print_task: Default::default(),
            bar_target,
            bars: Default::default(),

            prompt_task: Default::default(),
            pending_prompts: Default::default(),

            format_buffer: FormatBuffer::new(),
            buffered: String::new(),
        }
    }
}
impl Printer {
    pub(crate) fn set_colors(&mut self, use_color: bool) {
        self.colors = ansi::colors(use_color);
    }

    pub(crate) fn show_prompt(
        &mut self,
        prompt: &str,
        is_password: bool,
    ) -> oneshot::Receiver<std::io::Result<ZeroWhenDropString>> {
        // format the prompt
        let mut lines = prompt.lines();
        self.format_buffer.reset(self.colors.gray, self.colors.cyan);
        self.format_buffer.push_control(self.colors.cyan);
        self.format_buffer.push('!', 1);
        self.format_buffer.push(']', 1);
        if let Some(line) = lines.next() {
            self.format_buffer.push(' ', 1);
            self.format_buffer.push_str(line);
        }
        for line in lines {
            self.format_buffer.new_line();
            self.format_buffer.push_str(line);
        }
        if cfg!(feature = "prompt-password") && is_password {
            self.format_buffer.push_str(": ");
        } else {
            self.format_buffer.end();
            self.format_buffer.push_control(self.colors.reset);
            self.format_buffer.push_control("-: ");
        }

        // show the prompt
        let (send, recv) = oneshot::channel();
        if !self.prompt_task.active() {
            self.prompt_task.join();
            // erase current line, and print new prompt
            // this may mess up progress bars - having both prompts
            // and progress bar is not a good idea anyway
            use std::io::Write;
            let _ = write!(
                self.stdout,
                "{}{}{}",
                self.controls.move_to_begin_and_clear,
                self.buffered,
                self.format_buffer.as_str()
            );
            self.buffered.clear();
            let _ = self.stdout.flush();
            self.prompt_task.assign(prompt_task(send, is_password));
            return recv;
        }
        #[cfg(feature = "prompt-password")]
        {
            self.pending_prompts.push_back(PromptTask {
                send,
                prompt: self.format_buffer.take(),
                is_password,
            });
        }
        #[cfg(not(feature = "prompt-password"))]
        {
            self.pending_prompts.push_back(PromptTask {
                send,
                prompt: self.format_buffer.take(),
            });
        }
        recv
    }

    /// Spawn a progress bar, starting a print task if not already
    pub(crate) fn add_progress_bar(&mut self, bar: &Arc<ProgressBar>) {
        if lv::PRINT_LEVEL.get() < lv::Print::Quiet {
            return;
        }
        if self.bar_target.is_none() {
            return;
        }
        // start the bar
        self.bars.push(Arc::downgrade(bar));
        if !self.print_task.active() {
            self.print_task.join();
            // don't use bar if we can't measure terminal size
            let Some((width, height)) = super::term_width_height() else {
                return;
            };
            let max_bars = (height / 2).saturating_sub(2);
            // don't use bars if the terminal is too short
            if max_bars == 0 {
                return;
            }
            self.print_task.assign(print_task(width, max_bars as i32));
        }
    }

    /// Format and print the message
    pub(crate) fn print_message(&mut self, lv: lv::Lv, message: &str) {
        let mut lines = message.lines();
        let text_color = match lv {
            lv::Lv::Off => return,
            lv::Lv::Error => self.colors.red,
            lv::Lv::Hint => self.colors.yellow,
            lv::Lv::Print => self.colors.reset,
            lv::Lv::Warn => self.colors.yellow,
            lv::Lv::Info => self.colors.reset,
            lv::Lv::Debug => self.colors.cyan,
            lv::Lv::Trace => self.colors.magenta,
        };
        self.format_buffer.reset(self.colors.gray, text_color);
        match lv {
            lv::Lv::Off => unreachable!(),
            lv::Lv::Error => {
                self.format_buffer.push_control(self.colors.red);
                self.format_buffer.push('E', 1);
                self.format_buffer.push(']', 1);
            }
            lv::Lv::Hint => {
                self.format_buffer.push_control(self.colors.cyan);
                self.format_buffer.push('H', 1);
                self.format_buffer.push_control(self.colors.gray);
                self.format_buffer.push(']', 1);
            }
            lv::Lv::Print => {
                self.format_buffer.push_control(self.colors.gray);
                self.format_buffer.push(':', 1);
                self.format_buffer.push(':', 1);
            }
            lv::Lv::Warn => {
                self.format_buffer.push_control(self.colors.yellow);
                self.format_buffer.push('W', 1);
                self.format_buffer.push(']', 1);
            }
            lv::Lv::Info => {
                self.format_buffer.push_control(self.colors.green);
                self.format_buffer.push('I', 1);
                self.format_buffer.push_control(self.colors.gray);
                self.format_buffer.push(']', 1);
            }
            lv::Lv::Debug => {
                self.format_buffer.push_control(self.colors.gray);
                self.format_buffer.push('D', 1);
                self.format_buffer.push(']', 1);
            }
            lv::Lv::Trace => {
                self.format_buffer.push_control(self.colors.magenta);
                self.format_buffer.push('*', 1);
                self.format_buffer.push(']', 1);
            }
        }
        super::THREAD_NAME.with_borrow(|x| {
            if let Some(x) = x {
                self.format_buffer.push_control(self.colors.magenta);
                self.format_buffer.push('[', 1);
                self.format_buffer.push_str(x);
                self.format_buffer.push(']', 1);
            }
        });
        self.format_buffer.push_control(text_color);
        if let Some(line) = lines.next() {
            self.format_buffer.push(' ', 1);
            self.format_buffer.push_str(line);
        }
        for line in lines {
            self.format_buffer.new_line();
            self.format_buffer.push_str(line);
        }
        self.format_buffer.end();
        self.print_format_buffer();
    }

    /// Format and print a progress bar done message
    pub(crate) fn print_bar_done(&mut self, message: &str, is_progress_complete: bool) {
        if lv::PRINT_LEVEL.get() < lv::Print::Normal {
            return;
        }
        if is_progress_complete {
            self.format_buffer
                .reset(self.colors.gray, self.colors.green);
            self.format_buffer.push_control(self.colors.green);
        } else {
            self.format_buffer
                .reset(self.colors.gray, self.colors.yellow);
            self.format_buffer.push_control(self.colors.yellow);
        }
        self.format_buffer.push_str(message);
        self.format_buffer.end();
        self.print_format_buffer();
    }

    fn print_format_buffer(&mut self) {
        if !self.prompt_task.active() && self.bars.is_empty() {
            use std::io::Write;
            let _ = write!(self.stdout, "{}", self.format_buffer.as_str());
            let _ = self.stdout.flush();

            return;
        }
        self.buffered.push_str(self.format_buffer.as_str());
    }
    /// Take the buffered output and put it in buf
    fn take_buffered(&mut self, buf: &mut String) {
        buf.push_str(self.buffered.as_str());
        self.buffered.clear();
    }
    /// Print buffered output
    fn print_buffered(&mut self) {
        use std::io::Write as _;
        let _ = write!(self.stdout, "{}", self.buffered);
        self.buffered.clear();
        let _ = self.stdout.flush();
    }
    /// Print `buffer` to progress bar target
    fn print_to_bar_target(&mut self, buffer: &str) {
        use std::io::Write as _;
        match self.bar_target {
            None => {}
            Some(Target::Stdout) => {
                let _ = write!(self.stdout, "{buffer}");
                let _ = self.stdout.flush();
            }
            Some(Target::Stderr) => {
                let _ = write!(self.stderr, "{buffer}");
                let _ = self.stderr.flush();
            }
        }
    }

    pub(crate) fn take_print_task_if_should_join(&mut self) -> Option<JoinHandle<()>> {
        if self.print_task.needs_join {
            return self.print_task.take();
        }
        // if there are no bars, then eventually the task will end
        let strong_count = self.bars.iter().filter(|x| x.upgrade().is_some()).count();
        if strong_count == 0 {
            self.print_task.take()
        } else {
            None
        }
    }
    pub(crate) fn take_prompt_task_if_should_join(&mut self) -> Option<JoinHandle<()>> {
        if self.prompt_task.needs_join {
            return self.prompt_task.take();
        }

        if self.pending_prompts.is_empty() {
            self.prompt_task.take()
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq)]
enum Target {
    /// Print to Stdout
    Stdout,
    /// Print to Stderr
    Stderr,
}
#[derive(Default)]
struct PrintThread {
    needs_join: bool,
    handle: Option<JoinHandle<()>>,
}
impl PrintThread {
    /// Take the handle for joining
    fn take(&mut self) -> Option<JoinHandle<()>> {
        self.needs_join = false;
        self.handle.take()
    }

    /// Mark the task as will end, so it can be joined
    fn mark_join(&mut self) {
        self.needs_join = true;
    }

    /// If the task is active
    fn active(&self) -> bool {
        !self.needs_join && self.handle.is_some()
    }

    /// Blockingly join the task on the current thread
    fn join(&mut self) {
        self.needs_join = false;
        if let Some(handle) = self.handle.take() {
            let _: Result<_, _> = handle.join();
        }
    }

    /// Assign a new handle
    fn assign(&mut self, handle: JoinHandle<()>) {
        self.needs_join = false;
        self.handle = Some(handle);
    }
}

fn print_task(original_width: usize, max_bars: i32) -> JoinHandle<()> {
    use std::fmt::Write as _;

    // 50ms between each cycle
    const INTERVAL: Duration = Duration::from_millis(10);
    #[rustfmt::skip]
    const CHARS: [char; 30] = [
        '\u{280b}', '\u{280b}', '\u{280b}', '\u{280b}', '\u{280b}', 
        '\u{2819}', '\u{2819}', '\u{2819}', '\u{2819}', '\u{2819}', 
        '\u{2838}', '\u{2838}', '\u{2838}', '\u{2838}', '\u{2838}', 
        '\u{2834}', '\u{2834}', '\u{2834}', '\u{2834}', '\u{2834}', 
        '\u{2826}', '\u{2826}', '\u{2826}', '\u{2826}', '\u{2826}', 
        '\u{2807}', '\u{2807}', '\u{2807}', '\u{2807}', '\u{2807}',
    ];

    // main printer loop, also serves as RAII for printer lock
    // there are some issues with scope analysis and I am unsure
    // if drop() is working to prevent holding the lock during sleep
    #[inline(always)]
    fn print_loop(
        original_width: usize,
        max_bars: i32,
        tick: u32,
        buffer: &mut String,
        temp: &mut String,
        lines: &mut i32,
    ) -> std::ops::ControlFlow<()> {
        // This won't cause race condition where
        // the return value of start_print_task is put
        // into the handle after the task is ended,
        // because when calling start_print_task,
        // we have a lock on the printer, so this task
        // will wait until that lock is release to start
        #[inline(always)]
        fn on_task_end(printer: &mut Printer) {
            printer.print_task.mark_join();
        }
        #[inline(always)]
        fn clear(b: &mut String, lines: i32) {
            b.clear();
            b.push_str("\r\x1b[K"); // erase the last spacing line (... and X more)
            for _ in 0..lines {
                b.push_str("\x1b[1A\x1b[K"); // move up one line and erase it
            }
        }
        // std::thread::sleep(INTERVAL);
        clear(buffer, *lines);
        // scope for locking the printer
        let Ok(mut printer) = PRINTER.lock() else {
            return std::ops::ControlFlow::Break(());
        };
        if printer.bar_target.is_none() {
            on_task_end(&mut printer);
            return std::ops::ControlFlow::Break(());
        }

        if printer.prompt_task.active() {
            // don't do anything when there's a prompt,
            // since that will cause cursor to change position
            return std::ops::ControlFlow::Continue(());
        }
        let now = std::time::Instant::now();

        // remeasure terminal width on every cycle
        let width = super::term_width().unwrap_or(original_width);

        if printer.bar_target == Some(Target::Stdout) {
            // add the buffered messages
            printer.take_buffered(buffer);
        } else {
            printer.print_buffered();
        }
        // print the bars
        let mut more_bars = -max_bars;
        buffer.push_str(printer.colors.yellow);
        *lines = 0;
        let anime = CHARS[(tick as usize) % CHARS.len()];
        printer.bars.retain(|bar| {
            let Some(bar) = bar.upgrade() else {
                return false;
            };
            if more_bars < 0 {
                if width >= 2 {
                    buffer.push(anime);
                    buffer.push(']');
                    bar.format(width - 2, now, tick, INTERVAL, buffer, temp);
                }
                buffer.push('\n');
                *lines += 1;
            }
            more_bars += 1;

            true
        });

        if more_bars > 0 {
            temp.clear();
            if write!(temp, "  ... and {more_bars} more").is_err() {
                temp.clear();
            }
            if width >= temp.len() {
                buffer.push_str(temp);
                buffer.push_str(printer.colors.reset);
                buffer.push('\r');
            }
        } else {
            buffer.push_str(printer.colors.reset);
        }

        printer.print_to_bar_target(buffer);

        // check exit
        if printer.bars.is_empty() {
            on_task_end(&mut printer);
            // erase the bars
            clear(buffer, *lines);
            // we know the printer buffer is empty
            // because we just printed all of it while having
            // the lock on the printer
            printer.print_to_bar_target(buffer);
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    }

    std::thread::spawn(move || {
        // animation state
        let mut tick = 0;
        // buffer to avoid reallocation
        let mut temp = String::new();
        let mut buffer = String::new();
        // how many bars were printed
        let mut lines = 0;
        loop {
            match print_loop(
                original_width,
                max_bars,
                tick,
                &mut buffer,
                &mut temp,
                &mut lines,
            ) {
                std::ops::ControlFlow::Break(_) => break,
                _ => {
                    std::thread::sleep(INTERVAL);
                    tick = tick.wrapping_add(1);
                }
            };
        }
    })
}

// note that for interactive io, it's recommended to use blocking io directly
// on a thread instead of tokio
fn prompt_task(
    first_send: oneshot::Sender<std::io::Result<ZeroWhenDropString>>,
    _is_password: bool,
) -> JoinHandle<()> {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    std::thread::spawn(move || {
        let mut send = first_send;
        let mut _is_password = _is_password;
        let mut buf = String::new();
        loop {
            buf.clear();
            #[cfg(feature = "prompt-password")]
            let result = if _is_password {
                super::prompt_password::read_password()
            } else {
                std::io::stdin()
                    .read_line(&mut buf)
                    .map(|_| buf.clone().into())
            };
            #[cfg(not(feature = "prompt-password"))]
            let result = std::io::stdin()
                .read_line(&mut buf)
                .map(|_| buf.clone().into());
            let _ = send.send(result);
            let Ok(mut printer) = super::PRINTER.lock() else {
                break;
            };
            let Some(next) = printer.pending_prompts.pop_front() else {
                printer.prompt_task.mark_join();
                break;
            };
            let _ = write!(
                stdout,
                "{}{}{}",
                printer.controls.move_to_begin_and_clear, printer.buffered, next.prompt
            );
            printer.buffered.clear();
            let _ = stdout.flush();
            send = next.send;

            #[cfg(feature = "prompt-password")]
            {
                _is_password = next.is_password;
            }
        }
    })
}
