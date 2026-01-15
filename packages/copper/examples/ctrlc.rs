use pistonite_cu as cu;
use std::time::Duration;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    let check = cu::yesno!("should the sync version check for the signal?")?;
    sync_main(check)?;
    cu::co::run(async move {async_main().await})?;
    Ok(())
}

fn sync_main(check: bool) -> cu::Result<()> {
    match cu::cli::catch_ctrlc(move |ctrlc| {
        for _ in 0..30 {
            cu::print!("(sync) please press Ctrl-C");
            std::thread::sleep(Duration::from_millis(100));
            if check {
                ctrlc.check()?;
            }
        }
        cu::warn!("(sync) about to return!");
        cu::Ok(42)
    }){
        Ok(None) => cu::info!("(sync) was aborted!"),
        Ok(Some(n)) => cu::info!("(sync) was finished: {n}"),
        Err(e) => cu::error!("(sync) error: {e:?}"),
    }
    Ok(())
}

async fn async_main() -> cu::Result<()> {
    match cu::cli::co_catch_ctrlc(async |_| {
        for _ in 0..10 {
            cu::print!("(async) please press Ctrl-C");
            cu::co::sleep(Duration::from_secs(1)).await;
        }
        cu::warn!("(async) about to return!");
        cu::Ok(42)
    }).await {
        Ok(None) => cu::info!("(async) was aborted!"),
        Ok(Some(n)) => cu::info!("(async) was finished: {n}"),
        Err(e) => cu::error!("(async) error: {e:?}"),
    }
    Ok(())
}
