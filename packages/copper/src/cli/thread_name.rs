use std::cell::RefCell;

thread_local! {
    pub(crate) static THREAD_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the name to show up in messages printed by the current thread
#[inline(always)]
pub fn set_thread_name(name: impl Into<String>) {
    THREAD_NAME.with_borrow_mut(|x| *x = Some(name.into()))
}
