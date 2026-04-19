use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};

use cu::cli::printer::{PRINTER, Printer};
#[cfg(feature = "prompt")]
use cu::cli::prompt::PROMPT_LEVEL;
use cu::lv;
use env_filter::{Builder as LogEnvBuilder, Filter as LogEnvFilter};

static LOGGER: OnceLock<LogImpl> = OnceLock::new();

/// Shorthand to quickly setup logging. Can be useful in tests.
///
/// `qq`, `q`, `v` and `vv` inputs map to corresponding print levels. Other inputs
/// are mapped to default level
#[doc(alias = "quick_init")]
pub fn level(lv: &str) {
    let level = match lv {
        "qq" => lv::Print::QuietQuiet,
        "q" => lv::Print::Quiet,
        "v" => lv::Print::Verbose,
        "vv" => lv::Print::VerboseVerbose,
        _ => lv::Print::Normal,
    };
    init_options(
        lv::Color::Auto,
        level,
        Some(lv::Prompt::Block),
        Arc::new(DefaultLogConfig),
    );
}

/// Set global print options. This is usually called from clap args
///
/// If prompt option is `None`, it will be `Interactive` unless env var `CI` is `true` or `1`, in which case it becomes `No`.
/// Prompt option is ignored unless `prompt` feature is enabled
pub fn init_options(
    color: lv::Color,
    level: lv::Print,
    prompt: Option<lv::Prompt>,
    log_config: Arc<dyn LogConfig + Send + Sync>,
) {
    // not using cu::env_var, since we are before log initialization
    let env_rust_log = std::env::var("RUST_LOG");
    let (log_level_filter, log_filter) = match env_rust_log {
        Ok(value) if !value.is_empty() => {
            let filter = LogEnvBuilder::new().parse(&value).build();
            let log_level_filter = filter.filter();
            (log_level_filter.max(level.into()), Some(filter))
        }
        _ => (level.into(), None),
    };
    log::set_max_level(log_level_filter);

    let use_color = color.is_colored_for_stdout();
    lv::USE_COLOR.store(use_color, Ordering::Release);
    let printer = Printer::new(use_color);
    if let Ok(mut g_printer) = PRINTER.lock() {
        *g_printer = Some(printer);
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
        PROMPT_LEVEL.set(prompt)
    }
    #[cfg(not(feature = "prompt"))]
    {
        let _ = prompt;
    }

    lv::PRINT_LEVEL.set(level);

    let _ = LOGGER.set(LogImpl {
        filter: log_filter,
        config: log_config,
    });
    log::set_logger(LOGGER.get().unwrap()).unwrap();
}
struct LogImpl {
    filter: Option<LogEnvFilter>,
    config: Arc<dyn LogConfig + Send + Sync>,
}
impl log::Log for LogImpl {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        match &self.filter {
            Some(filter) => filter.enabled(metadata),
            None => lv::Lv::from(metadata.level()).can_print(lv::PRINT_LEVEL.get()),
        }
    }

    fn log(&self, record: &log::Record) {
        let (level, show_module) = self.config.process(record);
        if level != record.level().into() {
            let metadata = log::Metadata::builder()
                .level(level.into())
                .target(record.metadata().target())
                .build();
            if !self.enabled(&metadata) {
                return;
            }
        } else {
            if !self.enabled(record.metadata()) {
                return;
            }
        }
        let message = if show_module {
            // enable source location logging in trace messages
            let mut message = String::new();
            format_module_prefix(
                &mut message,
                record.module_path(),
                record.file(),
                record.line(),
            );
            use std::fmt::Write;
            let _: Result<_, _> = write!(&mut message, "{}", record.args());
            message
        } else {
            record.args().to_string()
        };
        if let Ok(mut printer) = PRINTER.lock() {
            if let Some(printer) = printer.as_mut() {
                printer.print_message(level, &message);
            }
        }
    }

    fn flush(&self) {}
}

fn format_module_prefix(
    message: &mut String,
    module: Option<&str>,
    file: Option<&str>,
    line: Option<u32>,
) {
    if module.is_none() && file.is_none() {
        return;
    }
    message.push('[');
    if let Some(p) = module {
        // aliased crate, use the shorthand
        if let Some(rest) = p.strip_prefix("pistonite_") {
            message.push_str(rest);
        } else {
            message.push_str(p);
        }
        message.push(' ');
    }
    if let Some(f) = file {
        let name = match f.rfind(['/', '\\']) {
            None => f,
            Some(i) => &f[i + 1..],
        };
        message.push_str(name);
        if let Some(l) = line {
            message.push(':');
            message.push_str(&format!("{l}"));
        }
    }
    message.push_str("] ");
}

/// Hook to configure the level and format before logging
pub trait LogConfig {
    /// Process a log record, return the level to log and if
    /// the module path should be shown
    fn process(&self, record: &lv::LogRecord) -> (lv::Lv, bool);
}
/// The default [`LogConfig`]
pub struct DefaultLogConfig;
impl LogConfig for DefaultLogConfig {
    fn process(&self, record: &lv::LogRecord) -> (lv::Lv, bool) {
        let level: lv::Lv = record.level().into();
        (record.level().into(), level == lv::T)
    }
}
