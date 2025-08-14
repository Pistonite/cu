use pistonite_cu as cu;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    let mut src = cu::fs::walk("src")?;
    cu::set_thread_print_name("walk");
    while let Some(entry) = src.next() {
        let entry = entry?;
        cu::info!("{} {}", entry.path().display(), entry.rel_path().display(),)
    }

    cu::set_thread_print_name("glob");
    let glob = cu::fs::glob_from("..", "./**/*.rs")?;
    for entry in glob {
        let entry = entry?;
        cu::info!("{}", entry.display());
    }

    Ok(())
}
