mod zstring;
pub use zstring::{ZString, zero_string};
mod byte_format;
pub use byte_format::ByteFormat;

// path requires fs since there are utils that checks for existence
// (check_exists, normalize)
#[cfg(feature = "fs")]
mod path;
#[cfg(feature = "fs")]
pub use path::{PathExtension, PathExtensionOwned};
