#[cfg(not(debug_assertions))]
use human_panic::setup_panic;

#[cfg(debug_assertions)]
extern crate better_panic;

pub(crate) mod cli;
pub(crate) mod core;
pub(crate) mod utils;

use crate::utils::app_config::AppConfig;
use crate::utils::error::Result;

/// The main entry point of the application.
fn main() -> Result<()> {
    // Human Panic. Only enabled when *not* debugging.
    #[cfg(not(debug_assertions))]
    {
        setup_panic!();
    }

    // Better Panic. Only enabled *when* debugging.
    #[cfg(debug_assertions)]
    {
        better_panic::Settings::debug()
            .most_recent_first(false)
            .lineno_suffix(true)
            .verbosity(better_panic::Verbosity::Full)
            .install();
    }

    let _guard = crate::utils::logger::setup_logging()?;

    // Initialize Configuration
    let config_contents = include_str!("resources/default_config.toml");
    AppConfig::init(Some(config_contents))?;

    // Match Commands
    crate::cli::cli_match()?;

    Ok(())
}
