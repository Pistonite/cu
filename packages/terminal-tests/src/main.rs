use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;

use cu::pre::*;

#[derive(clap::Parser)]
struct Cli {
    /// Only run this test
    #[clap(short, long)]
    test: Option<String>,
    /// Update snapshot
    #[clap(short, long)]
    update: bool,
    /// Display the output of the ith-case in the example instead of testing
    #[clap(short, long, requires = "test", conflicts_with = "update")]
    display: Option<usize>,

    /// Prompt instead of using pre-configured stdin
    #[clap(short = 'i', long, requires = "test", conflicts_with = "update")]
    inherit_stdin: bool,

    #[clap(flatten)]
    flags: cu::cli::Flags,
}

#[cu::cli(flags = "flags")]
async fn main(args: Cli) -> cu::Result<()> {
    match args.test {
        None => {
            let test_targets = cu::check!(find_tests(), "failed to find tests")?;
            run_test_targets(test_targets, args.update).await?;
        }
        Some(example_name) => {
            let path = crate_dir()
                .join("examples")
                .join(format!("{example_name}.rs"));
            let test_cases = cu::check!(
                parse_test_cases(&path),
                "failed to parse test case from '{example_name}'"
            )?;
            if let Some(case_i) = args.display {
                let test_case = cu::check!(
                    test_cases.get(case_i),
                    "index out of bound of test cases: {case_i}"
                )?;
                let childargs = &test_case.args;
                let stdin = test_case.stdin.as_ref().cloned().unwrap_or_default();
                let feature = format!("__test-{example_name},common");
                cu::print!("TEST OUTPUT >>>>>>>>>>>>>>>>>>>>>>>>>>");
                let command_builder = cu::which("cargo")?
                    .command()
                    // don't include warnings in the output
                    .env("RUSTFLAGS", "-Awarnings")
                    .args([
                        "run",
                        "-q",
                        "--example",
                        &example_name,
                        "--no-default-features",
                        "--features",
                        &feature,
                        "--",
                    ])
                    .args(childargs)
                    .stdout_inherit()
                    .stderr_inherit();
                let exit_status = if args.inherit_stdin {
                    command_builder.stdin_inherit().co_wait().await?
                } else {
                    command_builder
                        .stdin(cu::pio::write(stdin))
                        .co_wait()
                        .await?
                };
                cu::print!("TEST OUTPUT <<<<<<<<<<<<<<<<<<<<<<<<<<");
                cu::print!("STATUS: {exit_status}");
            } else {
                let test_target = TestTarget {
                    example_name,
                    test_cases,
                };
                run_test_targets(vec![test_target], args.update).await?;
            }
        }
    }
    Ok(())
}

struct TestTarget {
    example_name: String,
    test_cases: Vec<TestCase>,
}

struct TestCase {
    stdin: Option<Vec<u8>>,
    args: Vec<String>,
    skip: bool,
}

fn find_tests() -> cu::Result<Vec<TestTarget>> {
    let path = crate_dir().join("examples");

    // find example entry points
    let mut test_targets = vec![];
    let dir = cu::fs::read_dir(path)?;
    for entry in dir {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = cu::check!(name.to_str(), "not utf8")?;
        let Some(example_name) = name_str.strip_suffix(".rs") else {
            continue; // ignore non *.rs file
        };
        let test_cases = parse_test_cases(&entry.path())?;
        let test_target = TestTarget {
            example_name: example_name.to_string(),
            test_cases,
        };
        test_targets.push(test_target);
    }

    Ok(test_targets)
}

fn parse_test_cases(path: &Path) -> cu::Result<Vec<TestCase>> {
    let file = cu::fs::read_string(path)?;
    let mut test_cases = vec![];
    for line in file.lines() {
        let Some(line) = line.strip_prefix("// $") else {
            break;
        };
        let (line, skip) = match line.strip_prefix("-") {
            None => (line, false),
            Some(line) => (line, true),
        };
        let mut args = cu::check!(
            shell_words::split(line.trim()),
            "failed to parse command line: {line}"
        )?;
        let mut stdin = None;
        if args.len() >= 2 {
            if let Some("<") = args.get(args.len() - 2).map(|x| x.as_str()) {
                let stdin_path = args.pop().unwrap();
                let stdin_path = crate_dir().join("input").join(stdin_path);
                stdin = Some(cu::check!(
                    cu::fs::read(stdin_path),
                    "failed to read stdin for test case"
                )?);
                args.pop();
            }
        }
        test_cases.push(TestCase { args, stdin, skip });
    }
    Ok(test_cases)
}

async fn run_test_targets(targets: Vec<TestTarget>, update: bool) -> cu::Result<()> {
    let build_bar = cu::progress("building test targets")
        .total(targets.len())
        .eta(false)
        .spawn();
    // build one at a time
    let build_pool = cu::co::pool(1);
    let mut build_handles = Vec::with_capacity(targets.len());
    let mut total_tests = 0;
    for target in &targets {
        total_tests += target.test_cases.len();
        if target.test_cases.is_empty() {
            cu::warn!("no test case found in '{}'", target.example_name);
        }
        // cargo build --example X --features __test-X
        let example_name = target.example_name.clone();
        let build_bar = Arc::clone(&build_bar);
        let handle = build_pool.spawn(async move {
            let feature = format!("__test-{example_name},common");
            let (child, bar) = cu::which("cargo")?
                .command()
                .args([
                    "build",
                    "--example",
                    &example_name,
                    "--no-default-features",
                    "--features",
                    &feature,
                ])
                .preset(
                    cu::pio::cargo(format!("building {example_name}"))
                        .configure_spinner(|bar| bar.parent(Some(Arc::clone(&build_bar)))),
                )
                .co_spawn()
                .await?;
            child.co_wait_nz().await?;
            bar.done();
            cu::progress!(build_bar += 1);
            cu::Ok(example_name)
        });
        build_handles.push(handle);
    }
    drop(build_bar);

    let test_bar = cu::progress("running tests")
        .total(total_tests)
        .max_display_children(10)
        .eta(false)
        .spawn();
    let test_pool = cu::co::pool(-2);
    let mut test_handles = Vec::with_capacity(total_tests);
    let mut build_set = cu::co::set(build_handles);
    while let Some(result) = build_set.next().await {
        let example_name = result??;
        let target = cu::check!(
            targets.iter().find(|x| x.example_name == example_name),
            "unexpected: cannot find test cases"
        )?;

        for (index, test_case) in target.test_cases.iter().enumerate() {
            let example_name = example_name.clone();
            if test_case.skip {
                cu::warn!("skipping {example_name}-{index}");
                cu::progress!(test_bar += 1);
                continue;
            }
            let args = test_case.args.clone();
            let stdin = test_case.stdin.clone().unwrap_or_default();
            let test_bar = Arc::clone(&test_bar);

            let handle = test_pool.spawn(async move {
                let feature = format!("__test-{example_name},common");
                let command = shell_words::join(&args);
                let child_bar = test_bar.child(format!("{example_name}: {command}")).spawn();
                let (mut child, stdout, stderr) = cu::which("cargo")?
                    .command()
                    // don't include warnings in the output
                    .env("RUSTFLAGS", "-Awarnings")
                    .args([
                        "run",
                        "-q",
                        "--example",
                        &example_name,
                        "--no-default-features",
                        "--features",
                        &feature,
                        "--",
                    ])
                    .args(args)
                    .stdout(cu::pio::buffer())
                    .stderr(cu::pio::buffer())
                    .stdin(cu::pio::write(stdin))
                    .co_spawn()
                    .await?;
                let status = child.co_wait_timeout(Duration::from_secs(10)).await?;
                if status.is_none() {
                    child.co_kill().await?;
                }
                child_bar.done();
                let stdout = stdout.co_join().await??;
                let stderr = stderr.co_join().await??;
                let output = decode_output_streams(&command, stdout, stderr, status);

                cu::progress!(test_bar += 1);
                cu::Ok((example_name, command, index, output))
            });
            test_handles.push(handle);
        }
    }

    let mut test_set = cu::co::set(test_handles);
    let mut failures = vec![];
    while let Some(result) = test_set.next().await {
        let (example_name, command, index, output) = result??;
        let result = verify_output(&example_name, &command, index, &output, update);
        if let Err(error) = result {
            failures.push(error.to_string());
            cu::progress!(test_bar, "{} failed", failures.len());
        }
    }
    drop(test_bar);

    if failures.is_empty() {
        cu::info!("all tests passed");
        return Ok(());
    }

    for f in &failures {
        cu::warn!("test failed: {f}");
    }

    cu::bail!("{} tests failed", failures.len());
}

fn decode_output_streams(
    command: &str,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<ExitStatus>,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("$ ");
    out.push_str(command);
    out.push('\n');
    out.push_str("STDOUT >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>\n");
    decode_output_stream(&mut out, &stdout);
    out.push_str("^<EOF\n<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n");
    out.push_str("STDERR >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>\n");
    decode_output_stream(&mut out, &stderr);
    out.push_str("^<EOF\n<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n");

    match status {
        Some(status) => {
            let _ = write!(out, "status: {status}\n");
        }
        None => {
            let _ = write!(out, "timed out\n");
        }
    }

    out
}

fn decode_output_stream(out: &mut String, buffer: &[u8]) {
    for byte in buffer.iter().copied() {
        match byte {
            b' '..=b'~' => out.push(byte as char),
            b'\r' => out.push_str("^CR\n"),
            b'\n' => out.push_str("^LF\n"),
            byte => out.push_str(&format!("\\x{byte:02X}")),
        }
    }
}

fn verify_output(
    example_name: &str,
    command: &str,
    index: usize,
    output: &str,
    update: bool,
) -> cu::Result<()> {
    let mut output_path = crate_dir().join("output");
    let file_name = format!("{example_name}-{index}.txt");
    output_path.push(&file_name);

    if !output_path.exists() {
        cu::info!("new snapshot: {example_name}: {command}");
        cu::fs::write(output_path, output)?;
        return Ok(());
    }

    let expected_output = cu::fs::read_string(&output_path)?;
    // normalize line ending for windows users
    let expected_output = expected_output.lines().collect::<Vec<_>>().join("\n");
    if expected_output.trim() == output.trim() {
        cu::info!("pass: {example_name}: {command}");
        return Ok(());
    }

    if !update {
        cu::error!("fail: {example_name}: {command}");
        let mut wip_path = crate_dir().join("wip");
        wip_path.push(&file_name);
        cu::fs::write(wip_path, output)?;
        cu::bail!("output mismatch: {file_name}");
    }

    cu::fs::write(output_path, output)?;
    cu::info!("updated snapshot: {example_name}: {command}");
    Ok(())
}

fn crate_dir() -> &'static Path {
    env!("CARGO_MANIFEST_DIR").as_ref()
}
