use cu::pre::*;

#[cu::cli]
async fn main(_: cu::cli::Flags) -> cu::Result<()> {
    cu::info!("color test!");
    cu::which("eza")?
        .command()
        .stdoe(cu::lv::P)
        .stdin_null()
        .co_wait().await?;
    cu::which("eza")?
        .command()
        .add(cu::color_flag_eq())
        .add(cu::width_flag_eq())
        .stdoe(cu::lv::P)
        .stdin_null()
        .co_wait().await?;

    // spinner
    let cleanup = if cu::yesno!("run git test?")? {
        cu::info!("git test!");
        let child = cu::which("git")?
            .command()
            .args(["clone", "https://github.com/zeldaret/botw", "--progress"])
            .stdoe(cu::pio::spinner("cloning botw"))
            .stdin_null()
            .co_spawn().await?;
        let child2 = cu::which("git")?
            .command()
            .args([
                "clone",
                "https://github.com/rust-lang/rust",
                "--progress",
                "--depth",
                "1",
            ])
            .stdoe(cu::pio::spinner("cloning rust").info())
            .stdin_null()
            .co_spawn().await?;
        child.co_wait_nz().await?;
        child2.co_wait_nz().await?;
        cu::info!("done");
        cu::hint!("cleaning stuff up since you know i don't want to manually delete it");
        std::fs::remove_dir_all("rust")?;
        let handle1 = cu::co::co_spawn(async move {
            cu::fs::co_rec_remove("botw").await?;
            cu::Ok(())
        });
        let handle2 = cu::co::co_spawn(async move {
            cu::fs::co_rec_remove("rust").await?;
            cu::Ok(())
        });
        Some(cu::co::co_spawn(async move {
            handle1.co_join().await??;
            handle2.co_join().await??;
            cu::Ok(())
        }))
    } else {
        None
    };

    // pipes, co
    cu::co::co_spawn(async move {
        let (hello, out, _) = cu::which("echo")?
            .command()
            .arg("Hello, world!")
            .stdout(cu::pio::pipe())
            .stdie_null()
            .co_spawn()
            .await?;
        hello.co_wait_nz().await?;

        cu::which("rev")?
            .command()
            .stdin(out)
            .stdoe(cu::lv::I)
            .name("rev")
            .co_wait_nz()
            .await?;
        cu::Ok(())
    }).co_join().await??;

    // capture
    let (hello, out) = cu::which("cat")?
        .command()
        .arg("Cargo.toml")
        .stdout(cu::pio::string())
        .stdie_null()
        .co_spawn().await?;

    let x = out.co_join().await?;
    cu::info!("capture output: {x:?}");
    let x = x?;
    // let x=String::from_utf8(x)?;
    cu::info!("decoded: {x}");
    hello.co_wait_nz().await?;

    // blocking line stream
    let (child, lines, _) = cu::which("bash")?
        .command()
        .args(["-c", r#"for i in {1..5}; do echo "Line $i"; sleep 1; done"#])
        .stdout(cu::pio::lines())
        .stdie_null()
        .co_spawn().await?;
    // read the lines
    for line in lines {
        cu::info!("{line:?}");
    }
    child.co_wait_nz().await?;

    // async line stream
    let (child2, lines2) = cu::which("bash")?
        .command()
        .args(["-c", r#"for i in {1..5}; do echo "Line $i"; sleep 1; done"#])
        .stdout(cu::pio::co_lines())
        .stdie_null()
        .co_spawn().await?;
    // wait and read the lines
    let mut lines2 = lines2;
    while let Some(line) = lines2.next().await {
        let line = line?;
        cu::info!("{line:?}");
    }
    child2.co_wait_nz().await?;

    if let Some(cleanup) = cleanup {
        cleanup.co_join().await??
    }

    Ok(())
}
