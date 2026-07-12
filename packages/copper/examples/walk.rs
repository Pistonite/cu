use pistonite_cu as cu;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    let src = cu::fs::walk("src")?;
    cu::cli::set_thread_name("walk");
    for entry in src {
        let entry = entry?;
        cu::info!(
            "{} {} {:?}",
            entry.depth(),
            entry.path().display(),
            entry.rel_path()?
        )
    }

    cu::cli::set_thread_name("glob");
    let mut glob = cu::fs::walker("..");
    glob.glob_includes(["**/*.rs"])?;

    for entry in glob.walk()? {
        let entry = entry?;
        cu::info!("{entry:?}");
    }

    Ok(())
}
