// $- 0
// $- 1

use std::thread;
use std::time::Duration;

use cu::pre::*;

// spinner tests are skipped since the output can be unstable,
// depending on how the printing thread is scheduled

#[derive(clap::Parser, Clone, AsRef)]
struct Args {
    case: usize,
    #[clap(flatten)]
    #[as_ref]
    inner: cu::cli::Flags,
}
#[cu::cli]
fn main(args: Args) -> cu::Result<()> {
    cu::lv::disable_print_time();
    static CASES: &[fn() -> cu::Result<()>] = &[test_case_1, test_case_2];
    CASES[args.case]()
}

fn test_case_1() -> cu::Result<()> {
    // 3 sequential bars
    {
        // bar with message
        let bar = cu::progress("unbounded").spawn();
        cu::progress!(bar, "message1");
        sleep_tick();
        cu::progress!(bar, "message2");
        sleep_tick();
        cu::progress!(bar, "message3");
        sleep_tick();
        bar.done();
    }
    {
        // bar with progress
        let bar = cu::progress("finite").total(3).spawn();
        cu::progress!(bar += 1, "message1");
        sleep_tick();
        cu::progress!(bar += 1, "message2");
        sleep_tick();
        cu::progress!(bar += 1, "message3");
        sleep_tick();
    }
    {
        // bar with no keep
        let bar = cu::progress("finite, nokeep").keep(false).spawn();
        cu::progress!(bar = 1, "message1");
        sleep_tick();
        cu::progress!(bar = 2, "message2");
        sleep_tick();
        cu::progress!(bar = 3, "message3");
        sleep_tick();
    }
    Ok(())
}

fn test_case_2() -> cu::Result<()> {
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
            sleep_tick();

            if i == 5 {
                // this message should be displayed above the progress area
                cu::warn!("hi there");
                let answer = cu::prompt!("what's your favorite fruit?")?;
                cu::info!("{answer} is also my favorite fruit!");
            }
        }
        drop(bar4);
        drop(bar5);
        bar.done();
        for i in 0..10 {
            cu::progress!(bar2 += 1, "step {}", i + 10);
            sleep_tick();
            cu::print!("doing stuff");
        }
        cu::progress!(bar2 += 1, "last step");
    }

    cu::print!("bars done");

    Ok(())
}

fn sleep_tick() {
    thread::sleep(Duration::from_secs(1));
}
