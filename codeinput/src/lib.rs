#[cfg(feature = "types")]
mod core {
    pub mod types;
}

#[cfg(feature = "types")]
pub use core::types::*;

#[cfg(not(feature = "types"))]
pub mod core;
#[cfg(not(feature = "types"))]
pub mod utils;
