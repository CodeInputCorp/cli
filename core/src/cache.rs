use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::common::{
    collect_owners, collect_tags, find_owners_for_file, find_tags_for_file, get_repo_hash,
};
use crate::parse::parse_repo;
use crate::types::{CacheEncoding, CodeownersCache, CodeownersEntry, FileEntry};
use utils::error::{Error, Result};

/// Create a cache from parsed CODEOWNERS entries and files
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

/// Store Cache
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

/// Load Cache from file, automatically detecting whether it's JSON or Bincode format
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

pub fn sync_cache(
    repo: &std::path::Path, cache_file: Option<&std::path::Path>,
) -> Result<CodeownersCache> {
    let config_cache_file = utils::app_config::AppConfig::fetch()?.cache_file.clone();

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
        utils::error::Error::new(&format!(
            "Failed to load cache from {}: {}",
            cache_file.display(),
            e
        ))
    })?;

    // verify the hash of the cache matches the current repo hash
    let current_hash = get_repo_hash(repo)?;
    dbg!(current_hash);
    let cache_hash = cache.hash;
    dbg!(cache_hash);

    if cache_hash != current_hash {
        // parse the codeowners files and build the cache
        return parse_repo(&repo, &cache_file);
    } else {
        return Ok(cache);
    }
}
