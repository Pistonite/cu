use pistonite_cu as cu;

use cu::pre::*;

#[derive(clap::Parser)]
struct Cli {
    path: String,
    #[clap(flatten)]
    common: cu::cli::Flags,
}

#[cu::cli(flags = "common")]
fn main(args: Cli) -> cu::Result<()> {
    cu::info!("invoking cargo");
    let (child, bar) = cu::which("cargo")?
        .command()
        .args(["build"])
        .current_dir(args.path)
        .add(cu::color_flag())
        .preset(cu::pio::cargo("cargo build"))
        .spawn()?;
    child.wait_nz()?;
    bar.done();
    Ok(())
}
