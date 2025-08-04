use std::time::Duration;

#[derive(cu::cli::Parser, Clone)]
struct Args {
    #[clap(flatten)]
    inner: cu::cli::Flags,
}
/// Run with cargo run --example print --features prompt
#[cu::cli(flags = "inner")]
fn main(_: Args) -> cu::Result<()> {
    cu::print!("today's weather is {}", "good");
    cu::hint!("today's weather is {}", "ok");
    cu::info!(
        "this is an info messagenmultilineaa 你好 sldkfjals🤖kdjflkasjdflkjasldkfjaklsdjflkjasldkfjlaksjdflkajsdklfjlaksjdfkljasldkfjlasldkjflaskdjflaksjdlfkajsldkfjkasjdlfkjaskldjflajsdlkfjlaskjdfklajsdf"
    );
    cu::warn!("this is a warn message\n");
    cu::error!("this is error message\n\n");
    cu::debug!("this is debug message\n2\n\n");
    cu::trace!("this is trace message\n\n2\n");
    if !cu::yesno!("continue?")? {
        cu::warn!("you chose to not continue!");
        return Ok(());
    }
    cu::info!("you chose to continue!");

    {
        let bar2 = cu::progress_bar(20, "This takes 5 seconds");
        let bar = cu::progress_unbounded("This is unbounded");
        for i in 0..10 {
            cu::progress!(&bar, (), "step {i}");
            cu::progress!(&bar2, i, "step {i}");
            cu::debug!("this is debug message\n");
            std::thread::sleep(Duration::from_millis(250));
        }
        drop(bar);
        for i in 0..10 {
            cu::progress!(&bar2, i + 10, "step {}", i + 10);
            std::thread::sleep(Duration::from_millis(250));
            cu::print!("doing stuff");
        }
    }

    let thread1 = std::thread::spawn(|| {
        cu::set_thread_print_name("t1");
        let answer = cu::prompt!("from thread 1")?;
        cu::info!("you entered: {answer}");
        cu::Ok(())
    });
    let thread2 = std::thread::spawn(|| {
        cu::set_thread_print_name("t2");
        let answer = cu::prompt!("from thread 2")?;
        cu::info!("you entered: {answer}");
        cu::Ok(())
    });
    let thread3 = std::thread::spawn(|| {
        cu::set_thread_print_name("t3");
        let answer = cu::prompt!("from thread 3")?;
        cu::info!("you entered: {answer}");
        cu::Ok(())
    });
    let r1 = thread1.join().unwrap();
    let r2 = thread2.join().unwrap();
    let r3 = thread3.join().unwrap();
    r1?;
    r2?;
    r3?;
    cu::info!("all threads joined ok");

    Ok(())
}
