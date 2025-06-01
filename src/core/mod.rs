pub(crate) mod cache;
pub(crate) mod commands;
pub(crate) mod common;
pub mod owner_resolver;
pub(crate) mod parse;
pub mod parser;
pub mod resolver;
pub(crate) mod smart_iter;
pub mod tag_resolver;
pub mod types;

use crate::utils::error::Result;

pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
