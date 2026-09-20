use pistonite_cu as cu;

static GREEN: &str = "\x1b[92m";
static RED: &str = "\x1b[91m";
static YELLOW: &str = "\x1b[93m";
static RESET: &str = "\x1b[0m";

/// This logs P) in green
macro_rules! pass {
    ($($args:tt)*) => { cu::custom_print!(cu::lv::I, GREEN, 'P', "", ')', GREEN, $($args)*) }
}

/// This logs F) in red
macro_rules! fail {
    ($($args:tt)*) => { cu::custom_print!(cu::lv::E, RED, 'F', "", ')', RED, $($args)*) }
}

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    // cu::cli::print_to(cu::cli::Target::Stderr);
    let count = 5;
    pass!("{count} tests passed");
    fail!("{count} tests failed");
    // the part after rest should be red instead of no color
    // note because the escapes are formatted directly into this message,
    // they are not controllable by the --color flag
    fail!("{YELLOW}some{RESET} tests failed");
    cu::error!(
        "\x1b]8;;https://www.github.com\x07Click Me very very very very very very very very very long longl ongl laskdjfl aksjdlf kjasldkfj lasjdfl kasjdj asd lkfahsld khflaskdh f\x1b]8;;\x07"
    );
    cu::print!("\x1b]8;;https://www.github.com\x1b\\Click Me\x1b]8;;\x1b\\");
    Ok(())
}
