// $ -y --non-interactive
// $ --non-interactive
// $ -y < prompt-rust.txt
// $ < prompt-y-rust.txt
// $ < prompt-y-json.txt
// $ < prompt-n.txt
// $ < prompt-xn.txt

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    cu::lv::disable_print_time();
    cu::hint!("testing prompts");
    if !cu::yesno!("continue?")? {
        cu::warn!("you chose to not continue!");
        return Ok(());
    }
    let answer = cu::prompt!("what's your favorite programming language?")?;
    cu::info!("you answered: {answer}");
    if &*answer != "rust" {
        cu::bail!("the answer is incorrect");
    }
    cu::info!("the answer is correct");
    Ok(())
}
