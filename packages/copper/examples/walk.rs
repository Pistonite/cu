#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    let mut src = cu::fs::walk("src")?;
    while let Some(entry) = src.next() {
        let entry = entry?;
        cu::info!("{} {}", entry.path().display(), entry.rel_path().display(),)
    }

    Ok(())
}
