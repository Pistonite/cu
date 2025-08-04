use cu::pre::*;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    cu::info!("color test!");
    cu::which("eza")?
        .command()
        .stdoe(cu::lv::P)
        .stdin_null()
        .spawn()?
        .wait()?;
    cu::which("eza")?
        .command()
        .add(cu::color_flag_eq())
        .add(cu::width_flag_eq())
        .stdoe(cu::lv::P)
        .stdin_null()
        .spawn()?
        .wait()?;

    // spinner
    if cu::yesno!("run git test?")? {
        cu::info!("git test!");
        let child = cu::which("git")?
            .command()
            .args(["clone", "https://github.com/zeldaret/botw", "--progress"])
            .stdoe(cu::pio::spinner("cloning botw"))
            .stdin_null()
            .spawn()?;
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
            .spawn()?;
        child.wait_nz()?;
        child2.wait_nz()?;
        cu::info!("done");
        cu::hint!("cleaning stuff up since you know i don't want to manually delete it");
        std::fs::remove_dir_all("botw")?;
        std::fs::remove_dir_all("rust")?;
    }

    // pipes, co
    cu::co::run(async move {
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
    })?;

    // capture
    let (hello, out) = cu::which("cat")?
        .command()
        .arg("Cargo.toml")
        .stdout(cu::pio::string())
        .stdie_null()
        .spawn()?;

    let x = out.join()?;
    cu::info!("capture output: {x:?}");
    let x = x?;
    // let x=String::from_utf8(x)?;
    cu::info!("decoded: {x}");
    hello.wait_nz()?;

    // blocking line stream
    let (child, lines, _) = cu::which("bash")?
        .command()
        .args(["-c", r#"for i in {1..5}; do echo "Line $i"; sleep 1; done"#])
        .stdout(cu::pio::lines())
        .stdie_null()
        .spawn()?;
    // read the lines
    for line in lines {
        cu::info!("{line:?}");
    }
    child.wait_nz()?;

    // async line stream
    let (child2, lines2) = cu::which("bash")?
        .command()
        .args(["-c", r#"for i in {1..5}; do echo "Line $i"; sleep 1; done"#])
        .stdout(cu::pio::co_lines())
        .stdie_null()
        .spawn()?;
    // wait and read the lines
    cu::co::run(async move {
        let mut lines2 = lines2;
        while let Some(line) = lines2.next().await {
            let line = line?;
            cu::info!("{line:?}");
        }
        cu::Ok(())
    })?;
    child2.wait_nz()?;

    Ok(())
}
