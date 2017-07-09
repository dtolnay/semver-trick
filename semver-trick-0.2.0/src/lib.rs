/// This type is very widely used!! Interchangeable across 0.2 and 0.3.
pub struct Unchanged;

/// This type is not widely used. It will be removed in 0.3.0.
pub struct Removed;

/// This module contains a type that will be moved to a different module in
/// 0.3.0.
pub mod before {
    /// This type will be moved to a different module in 0.3.0.
    pub struct Moved;
}
