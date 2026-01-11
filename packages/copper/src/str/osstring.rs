use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use cu::Context as _;

/// Convenience trait for working with [`OsStr`](std::ffi::OsStr)
///
/// See [File System Paths and Strings](trait@crate::str::PathExtension)
pub trait OsStrExtension {
    /// Get the path as UTF-8, error if it's not UTF-8
    fn as_utf8(&self) -> cu::Result<&str>;
}
/// Convenience trait for working with [`OsString`](std::ffi::OsString)
///
/// See [File System Paths and Strings](trait@crate::str::PathExtension)
pub trait OsStrExtensionOwned {
    /// Get the path as UTF-8, error if it's not UTF-8
    fn into_utf8(self) -> cu::Result<String>;
}

impl OsStrExtension for OsStr {
    #[inline(always)]
    fn as_utf8(&self) -> cu::Result<&str> {
        cu::check!(self.to_str(), "not utf-8: {self:?}")
    }
}

impl OsStrExtension for Path {
    #[inline(always)]
    fn as_utf8(&self) -> cu::Result<&str> {
        self.as_os_str().as_utf8()
    }
}

impl OsStrExtensionOwned for OsString {
    #[inline(always)]
    fn into_utf8(self) -> cu::Result<String> {
        match self.into_string() {
            Ok(s) => Ok(s),
            Err(e) => cu::bail!("not utf-8: {e:?}"),
        }
    }
}
impl OsStrExtensionOwned for PathBuf {
    #[inline(always)]
    fn into_utf8(self) -> cu::Result<String> {
        self.into_os_string().into_utf8()
    }
}
#[cfg(all(test, unix))]
mod test {
    use std::os::unix::ffi::OsStrExt as _;
    use super::*;
    #[test]
    fn test_not_utf8() {
        let s = OsStr::from_bytes(b"hello\xffworld");
        let result = s.as_utf8().unwrap_err().to_string();
        assert_eq!(result, r#"not utf-8: "hello\xFFworld""#);
        let result = Path::new(s).as_utf8().unwrap_err().to_string();
        assert_eq!(result, r#"not utf-8: "hello\xFFworld""#);
    }
    #[test]
    fn test_utf8() {
        let s = OsStr::from_bytes(b"hello world");
        assert!(s.as_utf8().is_ok());
        assert!(Path::new(s).as_utf8().is_ok());
    }
}

#[cfg(all(test, windows))]
mod test {
    use std::os::windows::ffi::OsStringExt as _;
    use super::*;
    #[test]
    fn test_not_utf8() {
        let wide: &[u16] = &[0x0068, 0x0065, 0x006C, 0x006C, 0x006F, 0xD800, 0x0077, 0x006F, 0x0072, 0x006C, 0x0064];
        let s = OsString::from_wide(wide);
        let result = s.as_utf8().unwrap_err().to_string();
        assert_eq!(result, r#"not utf-8: "hello\u{d800}world""#);
        let result = Path::new(&s).as_utf8().unwrap_err().to_string();
        assert_eq!(result, r#"not utf-8: "hello\u{d800}world""#);

        let result = s.clone().into_utf8().unwrap_err().to_string();
        assert_eq!(result, r#"not utf-8: "hello\u{d800}world""#);
        let result = PathBuf::from(s).into_utf8().unwrap_err().to_string();
        assert_eq!(result, r#"not utf-8: "hello\u{d800}world""#);
    }
    #[test]
    fn test_utf8() {
        let wide: &[u16] = &[0x0068, 0x0065, 0x006C, 0x006C, 0x006F, 0x0020, 0x0077, 0x006F, 0x0072, 0x006C, 0x0064];
        let s = OsString::from_wide(wide);
        assert!(s.as_utf8().is_ok());
        assert!(Path::new(&s).as_utf8().is_ok());
        assert!(s.clone().into_utf8().is_ok());
        assert!(PathBuf::from(s).into_utf8().is_ok());
    }
}
