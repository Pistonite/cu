/// A string that will have its inner buffer zeroed when dropped
#[derive(Default, Clone)]
pub struct ZString(String);
impl ZString {
    pub const fn new() -> Self {
        Self(String::new())
    }
}
impl std::fmt::Display for ZString {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<String> for ZString {
    #[inline(always)]
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl AsRef<[u8]> for ZString {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl AsRef<String> for ZString {
    #[inline(always)]
    fn as_ref(&self) -> &String {
        &self.0
    }
}
impl AsRef<str> for ZString {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl Drop for ZString {
    #[inline(always)]
    fn drop(&mut self) {
        // safety: we are dropped
        unsafe { do_zero(&mut self.0) }
    }
}
impl std::ops::Deref for ZString {
    type Target = String;
    #[inline(always)]
    fn deref(&self) -> &String {
        &self.0
    }
}
impl std::ops::DerefMut for ZString {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Write 0's to the internal buffer of the string
#[inline(always)]
pub fn zero(s: &mut String) {
    let mut s = std::mem::take(s);
    // safety: s is dropped afterwards when going out of scope
    unsafe { do_zero(&mut s) }
}

// Safety: the string must be dropped afterwards
unsafe fn do_zero(s: &mut String) {
    // SAFETY: we don't use the string again
    for c in unsafe { s.as_bytes_mut() } {
        // SAFETY: c is a valid u8 pointer
        unsafe { std::ptr::write_volatile(c, 0) };
    }
    // ensure other threads see this change
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
