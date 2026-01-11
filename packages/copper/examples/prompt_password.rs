use pistonite_cu as cu;

// can only manually test this because password not prompted from stdin
#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    cu::hint!("testing prompt password");
    let answer = cu::prompt_password!("enter password")?;
    cu::info!("you answered: {answer}");
    Ok(())
}
