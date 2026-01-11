use pistonite_cu as cu;
use std::time::Duration;

use cu::pre::*;

#[derive(clap::Parser, Clone)]
struct Args {
    #[clap(flatten)]
    inner: cu::cli::Flags,
}
impl Args {
    fn preprocess(&mut self) {
        self.inner.verbose += 1;
        println!("{:#?}", self.inner);
    }
}
/// Run with cargo run --example print --features prompt,cli
#[cu::cli(flags = "inner", preprocess = Args::preprocess)]
fn main(_: Args) -> cu::Result<()> {
    if !cu::yesno!("continue?")? {
        cu::warn!("you chose to not continue!");
        return Ok(());
    }
    cu::info!("you chose to continue!");

    {
        let bar2 = cu::progress("This takes 5 seconds").total(20).spawn();
        let bar = bar2.child("This is unbounded").spawn();
        // make some fake hierarchy
        let bar3 = bar.child("level 2").total(3).keep(true).spawn();
        let bar4 = bar3.child("level 3").total(7).spawn();
        let bar5 = bar2.child("last").total(9).keep(true).spawn();
        for i in 0..10 {
            cu::progress!(bar, "step {i}");
            cu::progress!(bar2 = i, "step {i}");
            cu::progress!(bar3 += 1, "step {i}");
            cu::progress!(bar4 += 1, "step {i}");
            cu::progress!(bar5 += 1, "step {i}");
            cu::debug!("this is debug message\n");
            std::thread::sleep(Duration::from_millis(250));

            if i == 5 {
                cu::prompt!("what's your favorite fruit?")?;
            }
        }
        drop(bar4);
        drop(bar5);
        bar.done();
        for i in 0..10 {
            cu::progress!(bar2 += 1, "step {}", i + 10);
            std::thread::sleep(Duration::from_millis(250));
            cu::print!("doing stuff");
        }
        cu::progress!(bar2 += 1, "last step");
    }

    cu::print!("bars done");

    // let thread1 = std::thread::spawn(|| {
    //     cu::set_thread_print_name("t1");
    //     let answer = cu::prompt!("from thread 1")?;
    //     cu::info!("you entered: {answer}");
    //     cu::Ok(())
    // });
    // let thread2 = std::thread::spawn(|| {
    //     cu::set_thread_print_name("t2");
    //     let answer = cu::prompt!("from thread 2")?;
    //     cu::info!("you entered: {answer}");
    //     cu::Ok(())
    // });
    // let thread3 = std::thread::spawn(|| {
    //     cu::set_thread_print_name("t3");
    //     let answer = cu::prompt!("from thread 3")?;
    //     cu::info!("you entered: {answer}");
    //     cu::Ok(())
    // });
    // let r1 = thread1.join().unwrap();
    // let r2 = thread2.join().unwrap();
    // let r3 = thread3.join().unwrap();
    // r1?;
    // r2?;
    // r3?;
    // cu::info!("all threads joined ok");
    //
    // let command = cu::prompt!("enter command")?;
    // // note: in a real-world application, you would use something like
    // // the `shell_words` crate to split the input
    // let args: AnotherArgs = cu::check!(
    //     cu::cli::try_parse(command.split_whitespace()),
    //     "error parsing args"
    // )?;
    // cu::print!("parsed args: {args:?}");
    // // note: in a real-world application, this will probably be some subcommand
    // if args.help {
    //     cu::cli::print_help::<AnotherArgs>(true);
    // }

    Ok(())
}

/// Test Another Arg
///
/// long text here
#[derive(Debug, clap::Parser)]
#[clap(
    name = "",
    no_binary_name = true,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct AnotherArgs {
    /// the file
    ///
    /// long text here
    pub file: String,
    /// If we should copy
    ///
    /// long text here
    #[clap(short, long)]
    pub copy: bool,

    /// HELP ME
    #[clap(short, long, conflicts_with = "copy")]
    pub help: bool,
}
