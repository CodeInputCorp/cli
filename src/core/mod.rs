pub(crate) mod cache;
pub(crate) mod commands;
pub(crate) mod common;
pub(crate) mod parse;
pub mod parser;
pub(crate) mod resolver;
pub(crate) mod types;

use crate::utils::error::Result;

pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
