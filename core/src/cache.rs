use std::path::{Path, PathBuf};

use crate::common::{find_owners_for_file, find_tags_for_file};
use crate::types::{CodeownersCache, CodeownersEntry, FileEntry};
use utils::error::{Error, Result};

/// Create a cache from parsed CODEOWNERS entries and files
pub fn build_cache(entries: Vec<CodeownersEntry>, files: Vec<PathBuf>) -> Result<CodeownersCache> {
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

    Ok(CodeownersCache {
        entries,
        files: file_entries,
        owners_map,
        tags_map,
    })
}

/// Store Cache
pub fn store_cache(cache: &CodeownersCache, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("Invalid cache path"))?;
    std::fs::create_dir_all(parent)?;

    let file = std::fs::File::create(path)?;

    bincode::serialize_into(file, cache)
        .map_err(|e| Error::new(&format!("Failed to serialize cache: {}", e)))?;
}

/// Load cache from a file
pub fn load_cache(path: &Path) -> Result<CodeownersCache> {
    let file = std::fs::File::open(path)?;
    bincode::deserialize_from(file)
        .map_err(|e| Error::new(&format!("Failed to deserialize cache: {}", e)))
}

