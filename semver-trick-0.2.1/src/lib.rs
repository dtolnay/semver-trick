extern crate semver_trick;

pub use semver_trick::Unchanged;

/// This type is not widely used. It will be removed in 0.3.0.
pub struct Removed;

/// This module contains a type that will be moved to a different module in
/// 0.3.0.
pub mod before {
    pub use semver_trick::after::Moved;
}
