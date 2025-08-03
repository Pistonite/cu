use std::time::Duration;

use cu::pre::*;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    // cu::info!("git test!");
    // let child = cu::which("git")?
    // .command()
    // .args(["clone", "https://github.com/zeldaret/botw", "--progress"])
    //     .stdboth(cu::pio::spinner("cloning botw"))
    //     .stdin(cu::pio::null())
    //     .spawn()?;
    // let child2 = cu::which("git")?
    // .command()
    // .args(["clone", "https://github.com/rust-lang/rust", "--progress", "--depth", "1"])
    //     .stdboth(cu::pio::spinner("cloning rust").info())
    //     .stdin(cu::pio::null())
    //     .spawn()?;
    // child.wait_nz()?;
    // child2.wait_nz()?;
    // cu::info!("done");
    // cu::hint!("cleaning stuff up since you know i don't want to manually delete it");
    // std::fs::remove_dir_all("botw")?;
    // std::fs::remove_dir_all("rust")?;

    // pipes
    let mut hello = cu::which("echo")?.command()
    .arg("Hello, world!")
        .stdout(cu::pio::pipe())
        .stdin(cu::pio::null())
        .stderr(cu::pio::null())
    .spawn()?;

let reverse = cu::which("rev")?.command()
    .stdin(hello.stdout_mut().take()?)

    .stdboth(cu::lv::I)
        .name("rev")
        .spawn()?;

    hello.wait_nz()?;
    reverse.wait_nz()?;

    std::thread::sleep(Duration::from_millis(1000));

// assert_eq!(reverse.stdout, b"!dlrow ,olleH\n");
    Ok(())
}
