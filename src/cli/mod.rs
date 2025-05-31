use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use std::path::PathBuf;

// Module-level comments
//! # Command-Line Interface Module
//!
//! This module defines the command-line interface (CLI) for the application.
//! It uses the `clap` crate to parse arguments and subcommands, and then
//! dispatches to the appropriate handlers in the `core::commands` module.
//!
//! The main components are:
//! - `Cli`: The top-level struct representing the CLI arguments.
//! - `Commands`: An enum defining the main subcommands (e.g., `codeowners`, `completion`, `config`).
//! - `CodeownersSubcommand`: An enum for subcommands related to CODEOWNERS file management.
//! - `CompletionSubcommand`: An enum for generating shell completion scripts.
//! - `cli_match()`: The main function that parses CLI input and executes the matched command.
//! - `codeowners()`: A helper function to dispatch `CodeownersSubcommand` variants.

use crate::core::{
    commands,
    types::{CacheEncoding, OutputFormat},
};
use crate::utils::app_config::AppConfig;
use crate::utils::error::Result;
use crate::utils::types::LogLevel;

#[derive(Parser, Debug)]
#[command(
    name = "codeinput",
    author,
    about,
    long_about = "code input CLI",
    version
)]
//TODO: #[clap(setting = AppSettings::SubcommandRequired)]
//TODO: #[clap(global_setting(AppSettings::DeriveDisplayOrder))]
/// Represents the command-line interface arguments for the application.
///
/// This struct is parsed by `clap` to define the available commands, options, and flags.
pub struct Cli {
    /// Specifies a custom configuration file path.
    /// If not provided, the application will look for a default configuration file.
    /// TODO: parse(from_os_str)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enables or disables debug mode.
    /// This can affect logging verbosity and other debugging features.
    #[arg(name = "debug", short, long = "debug", value_name = "DEBUG")]
    pub debug: Option<bool>,

    /// Sets the logging level for the application.
    /// Valid options are typically defined in `LogLevel` enum (e.g., "error", "warn", "info", "debug", "trace").
    #[arg(
        name = "log_level",
        short,
        long = "log-level",
        value_name = "LOG_LEVEL"
    )]
    pub log_level: Option<LogLevel>,

    /// The subcommand to execute.
    /// This field holds one of the variants of the `Commands` enum.
    #[clap(subcommand)]
    command: Commands,
}

/// Defines the main subcommands available in the CLI.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Subcommands for managing and analyzing CODEOWNERS files.
    ///
    /// This command group provides tools for parsing, validating, and querying
    /// information from CODEOWNERS files.
    #[clap(
        name = "codeowners",
        about = "Manage and analyze CODEOWNERS files",
        long_about = "Tools for parsing, validating and querying CODEOWNERS files"
    )]
    Codeowners {
        /// The specific `CodeownersSubcommand` to execute.
        #[clap(subcommand)]
        subcommand: CodeownersSubcommand,
    },
    /// Subcommands for generating shell completion scripts.
    ///
    /// These commands allow users to generate autocompletion scripts for
    /// common shells like Bash, Zsh, and Fish, improving the usability of the CLI.
    #[clap(
        name = "completion",
        about = "Generate completion scripts",
        long_about = None,
        )]
    Completion {
        /// The specific `CompletionSubcommand` (shell type) for which to generate the script.
        #[clap(subcommand)]
        subcommand: CompletionSubcommand,
    },
    /// Displays the current application configuration.
    ///
    /// This command prints the active configuration, which is a result of merging
    /// default settings, configuration file values, and command-line arguments.
    #[clap(
        name = "config",
        about = "Show Configuration",
        long_about = None,
    )]
    Config,
}

/// Defines subcommands for shell completion script generation.
#[derive(Subcommand, PartialEq, Debug)]
enum CompletionSubcommand {
    /// Generates the autocompletion script for Bash.
    #[clap(about = "generate the autocompletion script for bash")]
    Bash,
    /// Generates the autocompletion script for Zsh.
    #[clap(about = "generate the autocompletion script for zsh")]
    Zsh,
    /// Generates the autocompletion script for Fish.
    #[clap(about = "generate the autocompletion script for fish")]
    Fish,
}

/// Defines subcommands related to CODEOWNERS file management.
#[derive(Subcommand, PartialEq, Debug)]
enum CodeownersSubcommand {
    /// Parses CODEOWNERS files and builds an ownership map.
    ///
    /// This command preprocesses CODEOWNERS files found within the specified path,
    /// resolves ownership rules, and creates a cache for faster lookups by other commands.
    #[clap(
        name = "parse",
        about = "Preprocess CODEOWNERS files and build ownership map"
    )]
    Parse {
        /// The directory path to analyze for CODEOWNERS files. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Specifies a custom location for the cache file. Defaults to `.codeowners.cache`.
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,

        /// The format for storing the cache: `json` or `bincode`. Defaults to `bincode`.
        #[arg(long, value_name = "FORMAT", default_value = "bincode", value_parser = parse_cache_encoding)]
        format: CacheEncoding,
    },

    /// Finds and lists files along with their owners based on specified filter criteria.
    ///
    /// This command queries the ownership information (potentially from a cache)
    /// to list files and their associated owners, allowing filtering by tags, owners,
    /// or unowned status.
    #[clap(
        name = "list-files",
        about = "Find and list files with their owners based on filter criteria"
    )]
    ListFiles {
        /// The directory path to analyze. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// Filters the list to show only files associated with the specified tags (comma-separated).
        #[arg(long, value_name = "LIST")]
        tags: Option<String>,

        /// Filters the list to show only files owned by the specified owners (comma-separated).
        #[arg(long, value_name = "LIST")]
        owners: Option<String>,

        /// If set, only lists files that have no owners defined in CODEOWNERS.
        #[arg(long)]
        unowned: bool,

        /// If set, shows all files, including those that are unowned or untagged.
        #[arg(long)]
        show_all: bool,

        /// The output format for the list: `text`, `json`, or `bincode`. Defaults to `text`.
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Specifies a custom location for the cache file. Defaults to `.codeowners.cache`.
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },

    /// Displays aggregated statistics and associations for owners.
    ///
    /// This command provides insights into owner activity, such as the number of files
    /// they own or other relevant metrics.
    #[clap(
        name = "list-owners",
        about = "Display aggregated owner statistics and associations"
    )]
    ListOwners {
        /// The directory path to analyze. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// The output format for the statistics: `text`, `json`, or `bincode`. Defaults to `text`.
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Specifies a custom location for the cache file. Defaults to `.codeowners.cache`.
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },
    /// Audits and analyzes the usage of tags across CODEOWNERS files.
    ///
    /// This command helps in understanding how tags are defined and used,
    /// potentially identifying unused or inconsistently applied tags.
    #[clap(
        name = "list-tags",
        about = "Audit and analyze tag usage across CODEOWNERS files"
    )]
    ListTags {
        /// The directory path to analyze. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// The output format for the tag analysis: `text`, `json`, or `bincode`. Defaults to `text`.
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Specifies a custom location for the cache file. Defaults to `.codeowners.cache`.
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },
}

/// Parses command-line arguments, merges configurations, and executes the appropriate command.
///
/// This is the main entry point for the CLI logic. It performs the following steps:
/// 1. Parses the raw command-line arguments using `Cli::parse()`.
/// 2. Merges any configuration specified via the `--config` option with `AppConfig`.
/// 3. Retrieves the `clap::Command` instance and its matches.
/// 4. Merges command-line arguments (which might override config file settings) into `AppConfig`.
/// 5. Matches the parsed subcommand and dispatches to the corresponding handler function
///    (e.g., `codeowners()` for `Codeowners` subcommands, or generates shell completions).
///
/// # Returns
///
/// Returns `Ok(())` on successful execution, or an `Err` variant from `crate::utils::error::Result`
/// if any step fails (e.g., config merging, argument parsing, command execution).
pub fn cli_match() -> Result<()> {
    // Parse the command line arguments
    let cli = Cli::parse();

    // Merge clap config file if the value is set
    AppConfig::merge_config(cli.config.as_deref())?;

    let app = Cli::command();
    let matches = app.get_matches();

    AppConfig::merge_args(matches)?;

    // Execute the subcommand
    match &cli.command {
        Commands::Codeowners { subcommand } => codeowners(subcommand)?,
        Commands::Completion { subcommand } => {
            let mut app = Cli::command();
            match subcommand {
                CompletionSubcommand::Bash => {
                    generate(Bash, &mut app, "codeinput", &mut std::io::stdout());
                }
                CompletionSubcommand::Zsh => {
                    generate(Zsh, &mut app, "codeinput", &mut std::io::stdout());
                }
                CompletionSubcommand::Fish => {
                    generate(Fish, &mut app, "codeinput", &mut std::io::stdout());
                }
            }
        }
        Commands::Config => commands::config()?,
    }

    Ok(())
}

/// Handles the dispatch of `CodeownersSubcommand` variants to their respective command functions.
///
/// This function takes a reference to a `CodeownersSubcommand` and calls the appropriate
/// function from `crate::core::commands` based on the variant.
///
/// # Arguments
///
/// * `subcommand`: A reference to the `CodeownersSubcommand` enum variant to be executed.
///
/// # Returns
///
/// Returns `Ok(())` if the subcommand executes successfully, or an `Err` variant
/// from `crate::utils::error::Result` if the command handler encounters an error.
pub(crate) fn codeowners(subcommand: &CodeownersSubcommand) -> Result<()> {
    match subcommand {
        CodeownersSubcommand::Parse {
            path,
            cache_file,
            format,
        } => commands::codeowners_parse(path, cache_file.as_deref(), *format),
        CodeownersSubcommand::ListFiles {
            path,
            tags,
            owners,
            unowned,
            show_all,
            format,
            cache_file,
        } => commands::codeowners_list_files(
            path.as_deref(),
            tags.as_deref(),
            owners.as_deref(),
            *unowned,
            *show_all,
            format,
            cache_file.as_deref(),
        ),
        CodeownersSubcommand::ListOwners {
            path,
            format,
            cache_file,
        } => commands::codeowners_list_owners(path.as_deref(), format, cache_file.as_deref()),
        CodeownersSubcommand::ListTags {
            path,
            format,
            cache_file,
        } => commands::codeowners_list_tags(path.as_deref(), format, cache_file.as_deref()),
    }
}

/// Parses a string slice into an `OutputFormat` enum.
///
/// This function is used by `clap` as a value parser for arguments
/// that specify an output format. It converts common string representations
/// (case-insensitive "text", "json", "bincode") into their corresponding
/// `OutputFormat` variants.
///
/// # Arguments
///
/// * `s`: The string slice to parse.
///
/// # Returns
///
/// Returns `Ok(OutputFormat)` if the string is a valid format, otherwise
/// returns `Err(String)` with an error message.
fn parse_output_format(s: &str) -> std::result::Result<OutputFormat, String> {
    match s.to_lowercase().as_str() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "bincode" => Ok(OutputFormat::Bincode),
        _ => Err(format!("Invalid output format: {}", s)),
    }
}

/// Parses a string slice into a `CacheEncoding` enum.
///
/// This function is used by `clap` as a value parser for arguments
/// that specify a cache encoding format. It converts common string representations
/// (case-insensitive "bincode", "json") into their corresponding
/// `CacheEncoding` variants.
///
/// # Arguments
///
/// * `s`: The string slice to parse.
///
/// # Returns
///
/// Returns `Ok(CacheEncoding)` if the string is a valid encoding, otherwise
/// returns `Err(String)` with an error message.
fn parse_cache_encoding(s: &str) -> std::result::Result<CacheEncoding, String> {
    match s.to_lowercase().as_str() {
        "bincode" => Ok(CacheEncoding::Bincode),
        "json" => Ok(CacheEncoding::Json),
        _ => Err(format!("Invalid cache encoding: {}", s)),
    }
}
