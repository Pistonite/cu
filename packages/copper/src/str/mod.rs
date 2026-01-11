mod zstring;
pub use zstring::{ZString, zero};
mod byte_format;
pub use byte_format::ByteFormat;

mod osstring;
pub use osstring::{OsStrExtension, OsStrExtensionOwned};


// path requires fs since there are utils that checks for existence
// (check_exists, normalize)
#[cfg(feature = "fs")]
mod path;
#[cfg(feature = "fs")]
pub use path::{PathExtension, PathExtensionOwned};
