#[macro_use]
extern crate log;

pub mod cache;
pub mod commands;
pub mod common;
pub mod types;

use utils::error::Result;

pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
