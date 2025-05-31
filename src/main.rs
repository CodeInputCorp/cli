#[cfg(not(debug_assertions))]
use human_panic::setup_panic;

#[cfg(debug_assertions)]
extern crate better_panic;

// Module-level comments
//! # Main Entry Point
//!
//! This file serves as the main entry point for the application. It is responsible for:
//! - Setting up panic handlers for improved error reporting.
//! - Initializing the logging system.
//! - Loading and initializing application configuration.
//! - Parsing and matching command-line arguments to execute corresponding commands.

pub(crate) mod cli;
pub(crate) mod core;
pub(crate) mod utils;

use crate::utils::app_config::AppConfig;
use crate::utils::error::Result;

/// The main entry point of the application.
///
/// This function orchestrates the startup of the application. Its key responsibilities include:
/// - Setting up panic handlers: `human_panic` for release builds and `better_panic` for debug builds.
/// - Initializing the logging infrastructure using `utils::logger::setup_logging`.
/// - Loading the application's configuration from `resources/default_config.toml`
///   and initializing the `AppConfig`.
/// - Parsing command-line arguments and dispatching to the appropriate command handlers
///   via `cli::cli_match`.
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
