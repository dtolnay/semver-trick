/// This type is very widely used!! Interchangeable across 0.2 and 0.3.
pub struct Unchanged;

/// This type has been added in 0.3.0.
pub struct Added;

/// This module contains a type that was previously in a different module.
pub mod after {
    /// This type will be moved to a different module in 0.3.0.
    pub struct Moved;
}
