// Module-level comments
//! # Repository Parsing Orchestration
//!
//! This module is responsible for the high-level orchestration of parsing a repository
//! to gather `CODEOWNERS` information and build a cache. It leverages utility functions
//! from `crate::core::common` for discovering and parsing individual `CODEOWNERS` files,
//! and `crate::core::cache` for building and storing the resulting `CodeownersCache`.
//!
//! The main entry point here, `parse_repo`, encapsulates the end-to-end process of:
//! 1. Finding all `CODEOWNERS` files within the repository.
//! 2. Parsing each of these files into `CodeownersEntry` objects.
//! 3. Discovering all other relevant files in the repository.
//! 4. Calculating a hash of the repository's current state.
//! 5. Building a `CodeownersCache` containing all this information.
//! 6. Storing the cache to a specified file.

use crate::utils::error::Result;

use super::{
    cache::{build_cache, store_cache},
    common::{find_codeowners_files, find_files, get_repo_hash, parse_codeowners},
    types::{CacheEncoding, CodeownersCache, CodeownersEntry},
};

/// Parses all `CODEOWNERS` files in a given repository, builds a cache, and stores it.
///
/// This function orchestrates the entire process of analyzing a repository for `CODEOWNERS`
/// information. It performs the following steps:
///
/// 1.  **Find `CODEOWNERS` Files**: Recursively searches the `repo` path for all files named `CODEOWNERS`
///     using `find_codeowners_files`.
/// 2.  **Parse `CODEOWNERS` Files**: Each found `CODEOWNERS` file is parsed into a collection of
///     `CodeownersEntry` structs using `parse_codeowners`. Entries from all files are aggregated.
/// 3.  **Find All Project Files**: All other files within the `repo` path (excluding `CODEOWNERS`
///     files themselves) are listed using `find_files`.
/// 4.  **Calculate Repository Hash**: A hash representing the current state of the repository
///     (HEAD commit, index, unstaged changes) is calculated using `get_repo_hash`. This hash
///     is stored in the cache for validation purposes.
/// 5.  **Build Cache**: A `CodeownersCache` is constructed using `build_cache`, containing the
///     parsed entries, the list of all project files, and the repository hash.
/// 6.  **Store Cache**: The newly built cache is serialized (currently using `Bincode` encoding)
///     and written to the location specified by `cache_file` (relative to the `repo` path)
///     using `store_cache`.
///
/// Informational messages are printed to the console during the process.
///
/// # Arguments
///
/// * `repo`: A `Path` reference to the root directory of the repository to be parsed.
/// * `cache_file`: A `Path` reference to the file where the generated cache should be stored.
///                 This path is typically relative to the `repo` path (e.g., ".codeowners.cache").
///
/// # Returns
///
/// Returns a `Result` containing the newly built `CodeownersCache` on success.
/// An `Error` is returned if any critical step fails, such as:
/// - Failure to read directories or files.
/// - Errors during the parsing of `CODEOWNERS` files.
/// - Inability to calculate the repository hash.
/// - Errors during cache building or storage.
pub fn parse_repo(repo: &std::path::Path, cache_file: &std::path::Path) -> Result<CodeownersCache> {
    println!("Parsing CODEOWNERS files at {}", repo.display());

    // Collect all CODEOWNERS files in the specified path
    let codeowners_files = find_codeowners_files(repo)?;

    // Parse each CODEOWNERS file and collect entries
    let parsed_codeowners: Vec<CodeownersEntry> = codeowners_files
        .iter()
        .filter_map(|file| {
            let parsed = parse_codeowners(file).ok()?;
            Some(parsed)
        })
        .flatten()
        .collect();

    // Collect all files in the specified path
    let files = find_files(repo)?;

    // Get the hash of the repository
    let hash = get_repo_hash(repo)?;

    // Build the cache from the parsed CODEOWNERS entries and the files
    let cache = build_cache(parsed_codeowners, files, hash)?;

    // Store the cache in the specified file
    store_cache(&cache, &repo.join(cache_file), CacheEncoding::Bincode)?;

    println!("CODEOWNERS parsing completed successfully");

    Ok(cache)
}
