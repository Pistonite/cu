use std::collections::VecDeque;
use std::io::{self, IsTerminal as _};
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex, Weak};
use std::thread::JoinHandle;

#[cfg(feature = "prompt")]
use oneshot::Receiver as OnceRecv;
use oneshot::Sender as OnceSend;

use crate::cli::fmt::{self, FormatBuffer, ansi};
#[cfg(feature = "prompt-password")]
use crate::cli::password;
use crate::cli::progress::{BarFormatter, BarResult, ProgressBar};
use crate::cli::{THREAD_NAME, TICK_INTERVAL, Tick};
use crate::lv;

/// Global printer state
pub(crate) static PRINTER: Mutex<Option<Printer>> = Mutex::new(None);
pub(crate) struct Printer {
    is_stdin_terminal: bool,
    /// Handle to stdout
    stdout: io::Stdout,
    /// Handle to stderr
    stderr: io::Stderr,
    /// Color codes
    colors: ansi::Colors,
    /// Control codes
    controls: ansi::Controls,

    print_task: PrintingThread,
    bar_target: Option<Target>,
    bars: Vec<Weak<ProgressBar>>,
    pending_prompts: VecDeque<PromptTask>,

    /// Buffer for automatically do certain formatting
    format_buffer: FormatBuffer,
    /// Place to buffer prints while printing is blocked
    buffered: String,
}
impl Printer {
    pub fn new(use_color: bool) -> Self {
        let colors = ansi::colors(use_color);
        let stdout = io::stdout();
        let stderr = io::stderr();
        let is_stdin_terminal = io::stdin().is_terminal();
        let (bar_target, is_terminal) = if cfg!(feature = "__test") {
            (Some(Target::Stdout), true)
        } else {
            let is_terminal = stdout.is_terminal();
            let bar_target = if is_terminal {
                Some(Target::Stdout)
            } else if stderr.is_terminal() {
                Some(Target::Stderr)
            } else {
                None
            };
            (bar_target, is_terminal)
        };
        let controls = ansi::controls(is_terminal);

        Self {
            is_stdin_terminal,
            stdout,
            stderr,
            colors,
            controls,

            print_task: Default::default(),
            bar_target,
            bars: Default::default(),
            pending_prompts: Default::default(),

            format_buffer: FormatBuffer::new(),
            buffered: String::new(),
        }
    }
    #[cfg(feature = "prompt")]
    pub(crate) fn show_prompt(
        &mut self,
        prompt: &str,
        is_password: bool,
    ) -> OnceRecv<io::Result<cu::ZString>> {
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
            self.format_buffer.push_lf();
            self.format_buffer.push_control(self.colors.reset);
            self.format_buffer.push_control("-: ");
        }

        let (send, recv) = oneshot::channel();
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
        self.start_print_task_if_needed();
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
        self.start_print_task_if_needed();
    }

    fn start_print_task_if_needed(&mut self) {
        if !self.print_task.active() {
            self.print_task.join();
            self.print_task.assign(print_task());
        }
    }
    /// Print a progress bar done message
    pub(crate) fn print_bar_done(&mut self, result: &BarResult, is_root: bool) {
        if lv::PRINT_LEVEL.get() < lv::Print::Normal {
            return;
        }
        if !is_root && self.bar_target.is_some() {
            // if bar is animated, don't print child's done messages
            return;
        }
        let message = match result {
            BarResult::DontKeep => return,
            BarResult::Done(message) => {
                self.format_buffer
                    .reset(self.colors.gray, self.colors.green);
                self.format_buffer.push_control(self.colors.green);
                message
            }
            BarResult::Interrupted(message) => {
                self.format_buffer
                    .reset(self.colors.gray, self.colors.yellow);
                self.format_buffer.push_control(self.colors.yellow);
                message
            }
        };
        self.format_buffer.push_control("\u{283f}]");
        if !message.starts_with('[') {
            self.format_buffer.push_control(" ");
        }
        self.format_buffer.push_str(message);
        self.format_buffer.push_lf();
        self.print_format_buffer();
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
        THREAD_NAME.with_borrow(|x| {
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
        self.format_buffer.push_lf();
        self.print_format_buffer();
    }
    fn print_format_buffer(&mut self) {
        if !self.print_task.active() {
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
        // if there are no bars and no prompts, then eventually the task will end
        // we have to check the strong count and not the bars size, because
        // we need to force the last bar to join the printing thread before
        // the program exits
        let bar_strong_count = self.bars.iter().filter(|x| x.upgrade().is_some()).count();
        if bar_strong_count == 0 && self.pending_prompts.is_empty() {
            self.print_task.take()
        } else {
            None
        }
    }
}

/// Printing thread that handles progress bar animation and printing during progress bar display
fn print_task() -> JoinHandle<()> {
    // progress bar animation chars
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
        tick: Tick,
        buffer: &mut String,
        temp: &mut String,
        lines: &mut i32,
    ) -> ControlFlow<()> {
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
        fn clear(b: &mut String, lines: &mut i32) {
            b.clear();
            b.push_str("\r\x1b[K"); // erase the last spacing line
            for _ in 0..*lines {
                b.push_str("\x1b[1A\x1b[K"); // move up one line and erase it
            }
            *lines = 0;
        }
        // first check if there are any pending prompts
        // scope for locking the printer for checking prompts
        {
            let Ok(mut printer_guard) = PRINTER.lock() else {
                return ControlFlow::Break(());
            };
            let Some(printer) = printer_guard.as_mut() else {
                return ControlFlow::Break(());
            };
            let task = printer.pending_prompts.pop_front();
            let is_stdin_terminal = printer.is_stdin_terminal;
            if let Some(mut task) = task {
                use std::io::Write as _;
                // print the prompt
                let control = printer.controls.move_to_begin_and_clear;
                let _ = write!(printer.stdout, "{}{}", control, task.prompt);
                let _ = printer.stdout.flush();

                // drop the lock while we wait for user input
                drop(printer_guard);
                // if there is a prompt, don't clear the previous progress bar yet,
                // since we want to display the prompt after the progress bars

                // we know the prompt string does not end with a new line (because of
                // the prompt prefix), so the number of lines to display
                // is exactly .lines().count()
                let mut l = task.prompt.lines().count() as i32;
                // however, if stdin is not terminal, then user won't press enter,
                // and we actually have 1 fewer line
                if !is_stdin_terminal {
                    l = l.saturating_sub(1)
                }
                *lines += l;
                // process this prompt
                #[cfg(feature = "prompt-password")]
                let (result, is_password) = if task.is_password {
                    (password::read_password(), true)
                } else {
                    (read_plaintext(temp), false)
                };
                #[cfg(not(feature = "prompt-password"))]
                let (result, is_password) = (read_plaintext(temp), false);

                // clear sensitive information in the memory
                crate::str::zero(temp);
                // now, re-print the prompt text to the buffer without the prompt prefix
                if !is_password {
                    while !task.prompt.ends_with('\n') {
                        task.prompt.pop();
                    }
                    task.prompt.pop(); // pop the final new line
                }
                // add the prompt to the print buffer
                if let Ok(mut printer) = PRINTER.lock() {
                    if let Some(printer) = printer.as_mut() {
                        printer.buffered.push_str(&task.prompt);
                        printer.buffered.push('\n');
                    }
                }
                // send the result of the prompt
                let _ = task.send.send(result);

                // we only process one prompt at a time
            }
        }

        // clear previous progress bars and prompts
        clear(buffer, lines);
        // lock the printer again for printing progress bars
        let Ok(mut printer) = PRINTER.lock() else {
            return ControlFlow::Break(());
        };
        let Some(printer) = printer.as_mut() else {
            return ControlFlow::Break(());
        };

        if let Some(bar_target) = printer.bar_target {
            // print the bars, after processing buffered messages

            // remeasure terminal width on every cycle
            let width = fmt::term_width_or_max();

            if bar_target == Target::Stdout {
                // add the buffered messages
                printer.take_buffered(buffer);
            } else {
                printer.print_buffered();
            }
            // print the bars
            buffer.push_str(printer.colors.yellow);
            let anime = CHARS[(tick as usize) % CHARS.len()];

            let mut formatter = BarFormatter {
                colors: printer.colors,
                bar_color: printer.colors.yellow,
                width,
                tick,
                now: &mut None,
                out: buffer,
                temp,
            };
            printer.bars.retain(|bar| {
                let Some(bar) = bar.upgrade() else {
                    // bar is done
                    return false;
                };
                if width >= 2 {
                    formatter.out.push(anime);
                    formatter.out.push(']');
                    *lines += bar.format(&mut formatter);
                } else {
                    formatter.out.push('\n');
                    *lines += 1;
                }

                true
            });
        }
        buffer.push_str(printer.colors.reset);
        printer.print_to_bar_target(buffer);
        let bars_empty = printer.bars.is_empty();
        let prompts_empty = printer.pending_prompts.is_empty();

        if bars_empty {
            // erase the bars
            clear(buffer, lines);
            printer.print_to_bar_target(buffer);
        }

        // check exit
        if bars_empty && prompts_empty {
            // nothing else to do, mark the task done,
            // so the printer knows to join this thread (after we drop the lock guard)
            // whenever someone calls, instead of posting to this thread
            on_task_end(printer);
            // we know the printer buffer is empty
            // because we just printed all of it while having
            // the lock on the printer, no need to print again
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
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
            match print_loop(tick, &mut buffer, &mut temp, &mut lines) {
                ControlFlow::Break(_) => break,
                _ => {
                    std::thread::sleep(TICK_INTERVAL);
                    tick = tick.wrapping_add(1);
                }
            };
        }
    })
}

fn read_plaintext(buf: &mut String) -> io::Result<cu::ZString> {
    buf.clear();
    io::stdin()
        .read_line(buf)
        .map(|_| buf.trim().to_string().into())
}

struct PromptTask {
    send: OnceSend<io::Result<cu::ZString>>,
    prompt: String,
    #[cfg(feature = "prompt-password")]
    is_password: bool,
}

/// For synchornizing with the printer
#[derive(Default)]
struct PrintingThread {
    needs_join: bool,
    /// Handle for the printing task, None means
    /// either no printing task is running, or, the printing
    /// task is terminating
    handle: Option<JoinHandle<()>>,
}
impl PrintingThread {
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Print to Stdout
    Stdout,
    /// Print to Stderr
    Stderr,
}
