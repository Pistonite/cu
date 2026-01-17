use pistonite_cu as cu;
use std::time::Duration;

#[cu::cli]
fn main(_: cu::cli::Flags) -> cu::Result<()> {
    sync_main()?;
    cu::co::run(async move { async_auto_check().await })?;
    cu::co::run(async move { async_manual_check().await })?;
    Ok(())
}

fn sync_main() -> cu::Result<()> {
    let result = cu::cli::ctrlc_frame().execute(move |ctrlc| {
        for _ in 0..30 {
            cu::print!("(sync) please press Ctrl-C");
            std::thread::sleep(Duration::from_millis(100));
            ctrlc.check()?;
        }
        cu::warn!("(sync) about to return!");
        cu::Ok(42)
    });
    match result {
        Ok(None) => cu::info!("(sync) was aborted!"),
        Ok(Some(n)) => cu::info!("(sync) was finished: {n}"),
        Err(e) => cu::error!("(sync) error: {e:?}"),
    }
    Ok(())
}

async fn async_manual_check() -> cu::Result<()> {
    let result = cu::cli::ctrlc_frame()
        .abort_threshold(3)
        .on_signal(|ctrlc| {
            if !ctrlc.should_abort() {
                cu::warn!("Ctrl-C 3 times to abort!")
            }
        })
        .co_execute(async |ctrlc| {
            for _ in 0..10 {
                cu::print!("(async) please press Ctrl-C");
                cu::co::sleep(Duration::from_secs(1)).await;
                if ctrlc.should_abort() {
                    cu::info!("just kidding, we never abort");
                }
            }
            cu::warn!("(async) about to return!");
            cu::Ok(42)
        })
        .await;
    match result {
        Ok(None) => cu::info!("(async) was aborted!"),
        Ok(Some(n)) => cu::info!("(async) was finished: {n}"),
        Err(e) => cu::error!("(async) error: {e:?}"),
    }
    Ok(())
}

async fn async_auto_check() -> cu::Result<()> {
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let result = cu::cli::ctrlc_frame()
        .on_signal(move |_| {
            let _ = send.send(());
        })
        .co_execute(async move |ctrlc| {
            let waiter = async move {
                loop {
                    if recv.recv().await.is_none() {
                        return;
                    }
                    if ctrlc.should_abort() {
                        return;
                    }
                }
            };
            tokio::select! {
                result = my_long_running_task() => {
                    result
                }
                _ = waiter => {
                    // does not matter what the value is -
                    // co_execute will ensure None is returned
                    // when aborted
                    Ok(0)
                }
            }
        })
        .await?;
    match result {
        Some(x) => cu::info!("valud is: {x}"),
        None => cu::error!("aborted!"),
    }
    Ok(())
}

async fn my_long_running_task() -> cu::Result<i32> {
    loop {
        cu::print!("will run forever if you don't abort");
        cu::co::sleep(Duration::from_secs(1)).await;
    }
}
