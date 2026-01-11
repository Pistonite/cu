/// return Ok if the error is abort
pub(crate) fn handle_join_error(e: tokio::task::JoinError) -> crate::Result<()> {
    let e = match e.try_into_panic() {
        Ok(panic) => {
            let info = crate::best_effort_panic_info(&panic);
            crate::bail!("task panicked: {info}");
        }
        Err(e) => e,
    };
    if e.is_cancelled() {
        Ok(())
    } else {
        crate::bail!("failed to join task due to unknown reason: {e:?}")
    }
}
