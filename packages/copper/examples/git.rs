use cu::pre::*;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    cu::info!("git test!");
    let child = cu::which("git")?
    .command()
    .args(["clone", "https://github.com/zeldaret/botw", "--progress"])
        .stdboth(cu::pio::spinner("cloning botw"))
        .stdin(cu::pio::null())
        .spawn()?;
    let child2 = cu::which("git")?
    .command()
    .args(["clone", "https://github.com/rust-lang/rust", "--progress", "--depth", "1"])
        .stdboth(cu::pio::spinner("cloning rust").info())
        .stdin(cu::pio::null())
        .spawn()?;
    child.wait_nz()?;
    child2.wait_nz()?;
    cu::info!("done");
    cu::hint!("cleaning stuff up since you know i don't want to manually delete it");
    std::fs::remove_dir_all("botw")?;
    std::fs::remove_dir_all("rust")?;
    Ok(())
}
