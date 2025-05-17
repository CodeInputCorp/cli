use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::common::{find_owners_for_file, find_tags_for_file};
use crate::types::{CacheEncoding, CodeownersCache, CodeownersEntry, FileEntry};
use utils::error::{Error, Result};

/// Create a cache from parsed CODEOWNERS entries and files
pub fn build_cache(entries: Vec<CodeownersEntry>, files: Vec<PathBuf>) -> Result<CodeownersCache> {
    let mut file_entries = Vec::new();
    let owners_map = std::collections::HashMap::new();
    let tags_map = std::collections::HashMap::new();

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
