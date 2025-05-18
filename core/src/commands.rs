use std::io::{self, Write};

use crate::cache::{build_cache, load_cache, store_cache, sync_cache};
use crate::common::{find_files, get_repo_hash};
use crate::parse::parse_repo;
use crate::types::{CacheEncoding, CodeownersCache, CodeownersEntry, OutputFormat};

use utils::app_config::AppConfig;
use utils::error::Result;

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}

/// Preprocess CODEOWNERS files and build ownership map
pub fn codeowners_parse(
    path: &std::path::Path, cache_file: Option<&std::path::Path>, encoding: CacheEncoding,
) -> Result<()> {
    println!("Parsing CODEOWNERS files at {}", path.display());

    let hash = get_repo_hash(path.as_ref())?;
    dbg!(hash);
    panic!();

    let cache_file = match cache_file {
        Some(file) => path.join(file),
        None => {
            let config = utils::app_config::AppConfig::fetch()?;
            path.join(config.cache_file)
        }
    };

    // Collect all CODEOWNERS files in the specified path
    let codeowners_files = crate::common::find_codeowners_files(path)?;

    // Parse each CODEOWNERS file and collect entries
    let parsed_codeowners: Vec<CodeownersEntry> = codeowners_files
        .iter()
        .filter_map(|file| {
            let parsed = crate::common::parse_codeowners(file).ok()?;
            Some(parsed)
        })
        .flatten()
        .collect();

    // Collect all files in the specified path
    let files = find_files(path)?;

    // Build the cache from the parsed CODEOWNERS entries and the files
    let hash = get_repo_hash(path)?;
    let cache = build_cache(parsed_codeowners, files, hash)?;

    // Store the cache in the specified file
    store_cache(&cache, &cache_file, encoding)?;

    // Test the cache by loading it back
    let _cache = load_cache(&cache_file)?;

    println!("CODEOWNERS parsing completed successfully");

    Ok(())
}

/// Find and list files with their owners based on filter criteria
pub fn codeowners_list_files(
    repo: Option<&std::path::Path>, tags: Option<&str>, owners: Option<&str>, unowned: bool,
    format: &OutputFormat, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Filter files based on criteria
    let filtered_files = cache
        .files
        .iter()
        .filter(|file| {
            // Check if we should include this file based on filters
            let passes_owner_filter = match owners {
                Some(owner_filter) => {
                    let owner_patterns: Vec<&str> = owner_filter.split(',').collect();
                    file.owners.iter().any(|owner| {
                        owner_patterns
                            .iter()
                            .any(|pattern| owner.identifier.contains(pattern))
                    })
                }
                None => true,
            };

            let passes_tag_filter = match tags {
                Some(tag_filter) => {
                    let tag_patterns: Vec<&str> = tag_filter.split(',').collect();
                    file.tags
                        .iter()
                        .any(|tag| tag_patterns.iter().any(|pattern| tag.0.contains(pattern)))
                }
                None => true,
            };

            let passes_unowned_filter = if unowned {
                file.owners.is_empty()
            } else {
                true
            };

            passes_owner_filter && passes_tag_filter && passes_unowned_filter
        })
        .collect::<Vec<_>>();

    // Output the filtered files in the requested format
    match format {
        OutputFormat::Text => {
            for file in filtered_files {
                let owners_str = file
                    .owners
                    .iter()
                    .map(|o| o.identifier.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                let tags_str = file
                    .tags
                    .iter()
                    .map(|t| t.0.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                println!("File: {}", file.path.display());
                println!(
                    "  Owners: {}",
                    if owners_str.is_empty() {
                        "None"
                    } else {
                        &owners_str
                    }
                );
                println!(
                    "  Tags: {}",
                    if tags_str.is_empty() {
                        "None"
                    } else {
                        &tags_str
                    }
                );
                println!();
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&filtered_files).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&filtered_files, bincode::config::standard())
                    .map_err(|e| {
                        utils::error::Error::new(&format!("Serialization error: {}", e))
                    })?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| utils::error::Error::new(&format!("IO error: {}", e)))?;
        }
    }

    Ok(())
}

/// Display aggregated owner statistics and associations
pub fn codeowners_list_owners(
    repo: Option<&std::path::Path>, format: &OutputFormat, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Process the owners from the cache
    match format {
        OutputFormat::Text => {
            println!("CODEOWNERS Ownership Report");
            println!("==========================\n");

            if cache.owners_map.is_empty() {
                println!("No owners found in the codebase.");
            } else {
                // Sort owners by number of files they own (descending)
                let mut owners_with_counts: Vec<_> = cache.owners_map.iter().collect();
                owners_with_counts.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                for (owner, paths) in owners_with_counts {
                    println!("Owner: {} ({})", owner.identifier, owner.owner_type);
                    println!("Files owned: {}", paths.len());

                    // List first 5 files (to avoid overwhelming output)
                    if !paths.is_empty() {
                        println!("Sample files:");
                        for path in paths.iter().take(5) {
                            println!("  - {}", path.display());
                        }

                        if paths.len() > 5 {
                            println!("  ... and {} more", paths.len() - 5);
                        }
                    }

                    println!(); // Empty line between owners
                }
            }
        }
        OutputFormat::Json => {
            // Convert to a more friendly JSON structure
            let owners_data: Vec<_> = cache.owners_map.iter()
                .map(|(owner, paths)| {
                    serde_json::json!({
                        "identifier": owner.identifier,
                        "type": format!("{:?}", owner.owner_type),
                        "file_count": paths.len(),
                        "files": paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
                    })
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&owners_data).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&cache.owners_map, bincode::config::standard())
                    .map_err(|e| {
                        utils::error::Error::new(&format!("Serialization error: {}", e))
                    })?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| utils::error::Error::new(&format!("IO error: {}", e)))?;
        }
    }

    println!(
        "Owners listing completed - {} owners found",
        cache.owners_map.len()
    );
    Ok(())
}

/// Audit and analyze tag usage across CODEOWNERS files
pub fn codeowners_list_tags(
    repo: Option<&std::path::Path>, format: &OutputFormat, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Process the tags from the cache
    match format {
        OutputFormat::Text => {
            println!("CODEOWNERS Tags Report");
            println!("======================\n");

            if cache.tags_map.is_empty() {
                println!("No tags found in the codebase.");
            } else {
                // Sort tags by number of files they're associated with (descending)
                let mut tags_with_counts: Vec<_> = cache.tags_map.iter().collect();
                tags_with_counts.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                for (tag, paths) in tags_with_counts {
                    println!("Tag: {}", tag.0);
                    println!("Files tagged: {}", paths.len());

                    // List first 5 files (to avoid overwhelming output)
                    if !paths.is_empty() {
                        println!("Sample files:");
                        for path in paths.iter().take(5) {
                            println!("  - {}", path.display());
                        }

                        if paths.len() > 5 {
                            println!("  ... and {} more", paths.len() - 5);
                        }
                    }

                    println!(); // Empty line between tags
                }
            }
        }
        OutputFormat::Json => {
            // Convert to a more friendly JSON structure
            let tags_data: Vec<_> = cache.tags_map.iter()
                .map(|(tag, paths)| {
                    serde_json::json!({
                        "name": tag.0,
                        "file_count": paths.len(),
                        "files": paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
                    })
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&tags_data).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&cache.tags_map, bincode::config::standard())
                    .map_err(|e| {
                        utils::error::Error::new(&format!("Serialization error: {}", e))
                    })?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| utils::error::Error::new(&format!("IO error: {}", e)))?;
        }
    }

    println!(
        "Tags listing completed - {} tags found",
        cache.tags_map.len()
    );
    Ok(())
}
