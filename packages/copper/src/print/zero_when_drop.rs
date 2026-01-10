/// A string that will have its inner buffer zeroed when dropped
#[derive(Default, Clone)]
pub struct ZeroWhenDropString(String);
impl ZeroWhenDropString {
    pub const fn new() -> Self {
        Self(String::new())
    }
}
impl std::fmt::Display for ZeroWhenDropString {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<String> for ZeroWhenDropString {
    #[inline(always)]
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl AsRef<[u8]> for ZeroWhenDropString {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl AsRef<String> for ZeroWhenDropString {
    #[inline(always)]
    fn as_ref(&self) -> &String {
        &self.0
    }
}
impl AsRef<str> for ZeroWhenDropString {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl Drop for ZeroWhenDropString {
    #[inline(always)]
    fn drop(&mut self) {
        zero_string(&mut self.0)
    }
}
impl std::ops::Deref for ZeroWhenDropString {
    type Target = String;
    #[inline(always)]
    fn deref(&self) -> &String {
        &self.0
    }
}
impl std::ops::DerefMut for ZeroWhenDropString {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) fn zero_string(s: &mut String) {
    let mut s = std::mem::take(s);
    // SAFETY: we don't use the string again
    for c in unsafe { s.as_bytes_mut() } {
        // SAFETY: c is a valid u8 pointer
        unsafe { std::ptr::write_volatile(c, 0) };
    }
    std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
