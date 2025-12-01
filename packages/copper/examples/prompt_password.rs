use pistonite_cu as cu;

use cu::pre::*;

#[derive(clap::Parser, Clone)]
struct Args {
    #[clap(flatten)]
    inner: cu::cli::Flags,
}

#[cu::cli(flags = "inner")]
fn main(_: Args) -> cu::Result<()> {
    let name = cu::prompt!("name")?;
    cu::info!("name is: {name}");
    // type for pw is ZeroWhenDropString
    let pw = cu::prompt_password!("password")?;
    cu::info!("password is: {pw}");
    Ok(())
}
