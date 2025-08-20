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
    cu::which("cargo")?
        .command()
        .args(["build"])
        .name("cargo build")
        .current_dir(args.path)
        .add(cu::color_flag())
        .preset(cu::pio::cargo())
        .spawn()?
        .0
        .wait_nz()?;
    Ok(())
}
