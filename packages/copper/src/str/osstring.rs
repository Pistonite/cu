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
    fn as_utf8(&self) -> crate::Result<&str> {
        self.as_os_str().as_utf8()
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

