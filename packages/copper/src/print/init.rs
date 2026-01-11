use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use crate::lv::{self, Lv};

static LOG_FILTER: OnceLock<env_filter::Filter> = OnceLock::new();
/// Set the global log filter
pub(crate) fn set_log_filter(filter: env_filter::Filter) {
    let _ = LOG_FILTER.set(filter);
}

/// Shorthand to quickly setup logging. Can be useful in tests.
///
/// "qq", "q", "v" and "vv" inputs map to corresponding print levels. Other inputs
/// are mapped to default level
pub fn log_init(lv: &str) {
    let level = match lv {
        "qq" => lv::Print::QuietQuiet,
        "q" => lv::Print::Quiet,
        "v" => lv::Print::Verbose,
        "vv" => lv::Print::VerboseVerbose,
        _ => lv::Print::Normal,
    };
    init_print_options(lv::Color::Auto, level, Some(lv::Prompt::Block));
}

/// Set global print options. This is usually called from clap args
///
/// If prompt option is `None`, it will be `Interactive` unless env var `CI` is `true` or `1`, in which case it becomes `No`.
/// Prompt option is ignored unless `prompt` feature is enabled
pub fn init_print_options(color: lv::Color, level: lv::Print, prompt: Option<lv::Prompt>) {
    let log_level = if let Ok(value) = std::env::var("RUST_LOG")
        && !value.is_empty()
    {
        let mut builder = env_filter::Builder::new();
        let filter = builder.parse(&value).build();
        let log_level = filter.filter();
        set_log_filter(filter);
        log_level.max(level.into())
    } else {
        level.into()
    };
    log::set_max_level(log_level);
    let use_color = color.is_colored_for_stdout();
    lv::USE_COLOR.store(use_color, Ordering::Release);
    if let Ok(mut printer) = super::PRINTER.lock() {
        printer.set_colors(use_color);
    }
    #[cfg(feature = "prompt")]
    {
        let prompt = match prompt {
            Some(x) => x,
            None => {
                let is_ci = std::env::var("CI")
                    .map(|mut x| {
                        x.make_ascii_lowercase();
                        matches!(x.trim(), "true" | "1")
                    })
                    .unwrap_or_default();
                if is_ci {
                    lv::Prompt::Block
                } else {
                    lv::Prompt::Interactive
                }
            }
        };
        super::PROMPT_LEVEL.set(prompt)
    }
    #[cfg(not(feature = "prompt"))]
    {
        let _ = prompt;
        super::PROMPT_LEVEL.set(lv::Prompt::No);
    }

    lv::PRINT_LEVEL.set(level);
    struct LogImpl;
    impl log::Log for LogImpl {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            match LOG_FILTER.get() {
                Some(filter) => filter.enabled(metadata),
                None => Lv::from(metadata.level()).can_print(lv::PRINT_LEVEL.get()),
            }
        }

        fn log(&self, record: &log::Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let typ: Lv = record.level().into();
            let message = if typ == Lv::Trace {
                // enable source location logging in trace messages
                let mut message = String::new();
                message.push('[');
                if let Some(p) = record.module_path() {
                    // aliased crate, use the shorthand
                    if let Some(rest) = p.strip_prefix("pistonite_") {
                        message.push_str(rest);
                    } else {
                        message.push_str(p);
                    }
                    message.push(' ');
                }
                if let Some(f) = record.file() {
                    let name = match f.rfind(['/', '\\']) {
                        None => f,
                        Some(i) => &f[i + 1..],
                    };
                    message.push_str(name);
                }
                if let Some(l) = record.line() {
                    message.push(':');
                    message.push_str(&format!("{l}"));
                }
                if message.len() > 1 {
                    message += "] ";
                } else {
                    message.clear();
                }

                use std::fmt::Write;
                let _: Result<_, _> = write!(&mut message, "{}", record.args());
                message
            } else {
                record.args().to_string()
            };
            if let Ok(mut printer) = super::PRINTER.lock() {
                printer.print_message(typ, &message);
            }
        }

        fn flush(&self) {}
    }

    let _ = log::set_logger(&LogImpl);
}

thread_local! {
    pub(crate) static THREAD_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the name to show up in messages printed by the current thread
pub fn set_thread_print_name(name: &str) {
    THREAD_NAME.with_borrow_mut(|x| *x = Some(name.to_string()))
}
