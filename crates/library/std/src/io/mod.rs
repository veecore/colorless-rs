//! Color-generic I/O primitives.

pub mod read;
pub mod unstable_rust;
pub mod util;
pub mod write;

pub use read::*;
pub use write::*;

pub type Result<T> = std::io::Result<T>;
pub type Error = std::io::Error;
