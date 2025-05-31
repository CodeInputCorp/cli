use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::common::{
    collect_owners, collect_tags, find_owners_for_file, find_tags_for_file, get_repo_hash,
};
use super::parse::parse_repo;
use super::types::{CacheEncoding, CodeownersCache, CodeownersEntry, FileEntry};
use crate::utils::error::{Error, Result};

// Module-level comments
//! # CODEOWNERS Cache Management
//!
//! This module provides functionalities for creating, storing, loading, and synchronizing
//! a cache of CODEOWNERS information. The cache helps in speeding up operations
//! like listing file owners or tags by avoiding repeated parsing of CODEOWNERS files.
//!
//! The main operations include:
//! - Building a new cache from parsed `CodeownersEntry` items and file lists.
//! - Storing a `CodeownersCache` object to a file, with support for different encodings.
//! - Loading a `CodeownersCache` from a file, automatically detecting the encoding.
//! - Synchronizing the cache, which involves checking if an existing cache is valid
//!   (e.g., by comparing a repository hash) and rebuilding it if necessary.

/// Creates a `CodeownersCache` from parsed CODEOWNERS entries, a list of files, and a repository hash.
///
/// This function processes each file to determine its owners and tags based on the provided
/// `CodeownersEntry` list. It also aggregates information about all unique owners and tags
/// found in the entries.
///
/// # Arguments
///
/// * `entries`: A vector of `CodeownersEntry` structs, representing the parsed rules from CODEOWNERS files.
/// * `files`: A vector of `PathBuf` pointing to the files in the repository that should be included in the cache.
/// * `hash`: A 32-byte array representing a hash of the repository state (e.g., commit hash or file content hash)
///           to validate cache freshness.
///
/// # Returns
///
/// Returns a `Result` containing the newly created `CodeownersCache` on success,
/// or an `Error` if any part of the cache building process fails (e.g., path processing).
pub fn build_cache(
    entries: Vec<CodeownersEntry>, files: Vec<PathBuf>, hash: [u8; 32],
) -> Result<CodeownersCache> {
    let mut file_entries = Vec::new();
    let mut owners_map = std::collections::HashMap::new();
    let mut tags_map = std::collections::HashMap::new();

    // Process each file to find owners and tags
    for file_path in files {
        let owners = find_owners_for_file(&file_path, &entries)?;
        let tags = find_tags_for_file(&file_path, &entries)?;

        // Build file entry
        let file_entry = FileEntry {
            path: file_path.clone(),
            owners: owners.clone(),
            tags: tags.clone(),
        };
        file_entries.push(file_entry);
    }

    // Process each owner
    let owners = collect_owners(&entries);
    owners.iter().for_each(|owner| {
        let paths = owners_map.entry(owner.clone()).or_insert_with(Vec::new);
        for file_entry in &file_entries {
            if file_entry.owners.contains(owner) {
                paths.push(file_entry.path.clone());
            }
        }
    });

    // Process each tag
    let tags = collect_tags(&entries);
    tags.iter().for_each(|tag| {
        let paths = tags_map.entry(tag.clone()).or_insert_with(Vec::new);
        for file_entry in &file_entries {
            if file_entry.tags.contains(tag) {
                paths.push(file_entry.path.clone());
            }
        }
    });

    Ok(CodeownersCache {
        hash,
        entries,
        files: file_entries,
        owners_map,
        tags_map,
    })
}

/// Stores a `CodeownersCache` object to a specified file path using the given encoding.
///
/// This function serializes the `CodeownersCache` into either Bincode or JSON format
/// and writes it to the file system. It also ensures that the parent directory for the
/// cache file exists, creating it if necessary.
///
/// # Arguments
///
/// * `cache`: A reference to the `CodeownersCache` to be stored.
/// * `path`: The `Path` where the cache file should be saved.
/// * `encoding`: The `CacheEncoding` to use (e.g., `Bincode` or `Json`).
///
/// # Returns
///
/// Returns `Ok(())` on successful storage, or an `Error` if directory creation,
/// file creation, serialization, or writing fails.
pub fn store_cache(cache: &CodeownersCache, path: &Path, encoding: CacheEncoding) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("Invalid cache path"))?;
    std::fs::create_dir_all(parent)?;

    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);

    match encoding {
        CacheEncoding::Bincode => {
            bincode::serde::encode_into_std_write(cache, &mut writer, bincode::config::standard())
                .map_err(|e| Error::new(&format!("Failed to serialize cache: {}", e)))?;
        }
        CacheEncoding::Json => {
            serde_json::to_writer_pretty(&mut writer, cache)
                .map_err(|e| Error::new(&format!("Failed to serialize cache to JSON: {}", e)))?;
        }
    }

    writer.flush()?;

    Ok(())
}

/// Loads a `CodeownersCache` from a specified file path, automatically detecting the encoding.
///
/// This function attempts to determine if the cache file is in JSON or Bincode format.
/// It first checks if the file starts with `'{'`, which suggests JSON. If so, it tries
/// to deserialize it as JSON. Otherwise, it attempts Bincode deserialization. If Bincode
/// fails and it wasn't identified as JSON initially, it makes a fallback attempt to
/// deserialize as JSON.
///
/// # Arguments
///
/// * `path`: The `Path` to the cache file to be loaded.
///
/// # Returns
///
/// Returns a `Result` containing the loaded `CodeownersCache` on success,
/// or an `Error` if the file cannot be opened, read, or deserialized in any
/// supported format.
pub fn load_cache(path: &Path) -> Result<CodeownersCache> {
    // Read the first byte to make an educated guess about the format
    let mut file = std::fs::File::open(path)
        .map_err(|e| Error::new(&format!("Failed to open cache file: {}", e)))?;

    let mut first_byte = [0u8; 1];
    let read_result = file.read_exact(&mut first_byte);

    // Close the file handle and reopen for full reading
    drop(file);

    if read_result.is_ok() && first_byte[0] == b'{' {
        // First byte is '{', likely JSON
        let file = std::fs::File::open(path)
            .map_err(|e| Error::new(&format!("Failed to open cache file: {}", e)))?;
        let reader = std::io::BufReader::new(file);

        return serde_json::from_reader(reader)
            .map_err(|e| Error::new(&format!("Failed to deserialize JSON cache: {}", e)));
    }

    // Try bincode first since it's not JSON
    let file = std::fs::File::open(path)
        .map_err(|e| Error::new(&format!("Failed to open cache file: {}", e)))?;
    let mut reader = std::io::BufReader::new(file);

    match bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard()) {
        Ok(cache) => Ok(cache),
        Err(_) => {
            // If bincode fails and it's not obviously JSON, still try JSON as a fallback
            let file = std::fs::File::open(path)
                .map_err(|e| Error::new(&format!("Failed to open cache file: {}", e)))?;
            let reader = std::io::BufReader::new(file);

            serde_json::from_reader(reader).map_err(|e| {
                Error::new(&format!(
                    "Failed to deserialize cache in any supported format: {}",
                    e
                ))
            })
        }
    }
}

/// Synchronizes the CODEOWNERS cache for a given repository.
///
/// This function checks if a valid cache file exists and matches the current state of the
/// repository (verified by a hash). If the cache is missing, outdated, or invalid,
/// it triggers a re-parse of the repository's CODEOWNERS files and rebuilds the cache.
///
/// The location of the cache file can be specified directly or retrieved from the
/// application configuration.
///
/// # Arguments
///
/// * `repo`: A `Path` to the root of the repository to be analyzed.
/// * `cache_file`: An optional `Path` to the cache file. If `None`, the path is
///                 determined from `AppConfig`.
///
/// # Returns
///
/// Returns a `Result` containing the `CodeownersCache` (either loaded or newly built)
/// on success, or an `Error` if loading, parsing, or cache building fails.
pub fn sync_cache(
    repo: &std::path::Path, cache_file: Option<&std::path::Path>,
) -> Result<CodeownersCache> {
    let config_cache_file = crate::utils::app_config::AppConfig::fetch()?
        .cache_file
        .clone();

    let cache_file: &std::path::Path = match cache_file {
        Some(file) => file.into(),
        None => std::path::Path::new(&config_cache_file),
    };

    // Verify that the cache file exists
    if !repo.join(cache_file).exists() {
        // parse the codeowners files and build the cache
        return parse_repo(&repo, &cache_file);
    }

    // Load the cache from the specified file
    let cache = load_cache(&repo.join(cache_file)).map_err(|e| {
        crate::utils::error::Error::new(&format!(
            "Failed to load cache from {}: {}",
            cache_file.display(),
            e
        ))
    })?;

    // verify the hash of the cache matches the current repo hash
    let current_hash = get_repo_hash(repo)?;
    let cache_hash = cache.hash;

    if cache_hash != current_hash {
        // parse the codeowners files and build the cache
        return parse_repo(&repo, &cache_file);
    } else {
        return Ok(cache);
    }
}
