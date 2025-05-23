use std::io::{self, Write};

use super::cache::{build_cache, load_cache, store_cache, sync_cache};
use super::common::{find_files, get_repo_hash};
use super::types::{CacheEncoding, CodeownersEntry, OutputFormat};

use crate::utils::app_config::AppConfig;
use crate::utils::error::{Error, Result};

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

    let cache_file = match cache_file {
        Some(file) => path.join(file),
        None => {
            let config = AppConfig::fetch()?;
            path.join(config.cache_file)
        }
    };

    // Collect all CODEOWNERS files in the specified path
    let codeowners_files = super::common::find_codeowners_files(path)?;

    // Parse each CODEOWNERS file and collect entries
    let parsed_codeowners: Vec<CodeownersEntry> = codeowners_files
        .iter()
        .filter_map(|file| {
            let parsed = super::common::parse_codeowners(file).ok()?;
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
            // Set column widths that work better for most displays
            let path_width = 45; // Max width for path display
            let owner_width = 26; // More space for owners
            let tag_width = 26; // More space for tags

            // Print header
            println!(
                "==============================================================================="
            );
            println!(
                " {:<path_width$} {:<owner_width$} {:<tag_width$}",
                "File Path",
                "Owners",
                "Tags",
                path_width = path_width,
                owner_width = owner_width,
                tag_width = tag_width
            );
            println!(
                "==============================================================================="
            );

            // Print each file entry
            for file in &filtered_files {
                // Format the path - keep the filename but truncate the path if needed
                let path_str = file.path.to_string_lossy();
                let path_display = if path_str.len() > path_width {
                    // Extract filename
                    let filename = file
                        .path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // Calculate available space for parent path
                    let available_space = path_width.saturating_sub(filename.len() + 4); // +4 for ".../"

                    if available_space > 5 {
                        // Show part of the parent path
                        let parent_path = path_str.to_string();
                        let start_pos = parent_path.len().saturating_sub(path_width - 3);
                        format!("...{}", &parent_path[start_pos..])
                    } else {
                        // Just show the filename with ellipsis
                        format!(".../{}", filename)
                    }
                } else {
                    path_str.to_string()
                };

                // Format owners with more space
                let owners_str = if file.owners.is_empty() {
                    "None".to_string()
                } else {
                    file.owners
                        .iter()
                        .map(|o| o.identifier.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let owners_display = if owners_str.len() > owner_width {
                    format!("{}...", &owners_str[0..owner_width - 3])
                } else {
                    owners_str
                };

                // Format tags with more space
                let tags_str = if file.tags.is_empty() {
                    "None".to_string()
                } else {
                    file.tags
                        .iter()
                        .map(|t| t.0.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let tags_display = if tags_str.len() > tag_width {
                    format!("{}...", &tags_str[0..tag_width - 3])
                } else {
                    tags_str
                };

                println!(
                    " {:<path_width$} {:<owner_width$} {:<tag_width$}",
                    path_display,
                    owners_display,
                    tags_display,
                    path_width = path_width,
                    owner_width = owner_width,
                    tag_width = tag_width
                );
            }
            println!(
                "==============================================================================="
            );
            println!(" Total: {} files", filtered_files.len());
            println!(
                "==============================================================================="
            );
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&filtered_files).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&filtered_files, bincode::config::standard())
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
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
            // Column widths for the table
            let owner_width = 35; // For owner identifiers
            let type_width = 10; // For owner type
            let count_width = 10; // For file count
            let file_width = 45; // For sample files

            println!(
                "==============================================================================="
            );
            println!(
                " {:<owner_width$} {:<type_width$} {:<count_width$} {:<file_width$}",
                "Owner",
                "Type",
                "Files",
                "Sample Files",
                owner_width = owner_width,
                type_width = type_width,
                count_width = count_width,
                file_width = file_width
            );
            println!(
                "==============================================================================="
            );

            if cache.owners_map.is_empty() {
                println!(" No owners found in the codebase.");
            } else {
                // Sort owners by number of files they own (descending)
                let mut owners_with_counts: Vec<_> = cache.owners_map.iter().collect();
                owners_with_counts.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                for (owner, paths) in owners_with_counts {
                    // Prepare sample file list
                    let file_samples = if paths.is_empty() {
                        "None".to_string()
                    } else {
                        let samples: Vec<_> = paths
                            .iter()
                            .take(3) // Show max 3 files as samples
                            .map(|p| {
                                let file_name = p
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                                file_name
                            })
                            .collect();
                        let mut display = samples.join(", ");
                        if paths.len() > 3 {
                            display.push_str(&format!(" (+{})", paths.len() - 3));
                        }
                        display
                    };

                    // Trim the owner identifier if too long
                    let owner_display = if owner.identifier.len() > owner_width {
                        format!("{}...", &owner.identifier[0..owner_width - 3])
                    } else {
                        owner.identifier.clone()
                    };

                    println!(
                        " {:<owner_width$} {:<type_width$} {:<count_width$} {:<file_width$}",
                        owner_display,
                        owner.owner_type,
                        paths.len(),
                        file_samples,
                        owner_width = owner_width,
                        type_width = type_width,
                        count_width = count_width,
                        file_width = file_width
                    );
                }
            }
            println!(
                "==============================================================================="
            );
            println!(" Total: {} owners", cache.owners_map.len());
            println!(
                "==============================================================================="
            );
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
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
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
            // Column widths for the table
            let tag_width = 30; // For tag name
            let count_width = 10; // For file count
            let files_width = 60; // For sample files

            println!(
                "==============================================================================="
            );
            println!(
                " {:<tag_width$} {:<count_width$} {:<files_width$}",
                "Tag",
                "Files",
                "Sample Files",
                tag_width = tag_width,
                count_width = count_width,
                files_width = files_width
            );
            println!(
                "==============================================================================="
            );

            if cache.tags_map.is_empty() {
                println!(" No tags found in the codebase.");
            } else {
                // Sort tags by number of files they're associated with (descending)
                let mut tags_with_counts: Vec<_> = cache.tags_map.iter().collect();
                tags_with_counts.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

                for (tag, paths) in tags_with_counts {
                    // Prepare sample file list - show filenames only, not full paths
                    let file_samples = if paths.is_empty() {
                        "None".to_string()
                    } else {
                        let samples: Vec<_> = paths
                            .iter()
                            .take(5) // Show max 5 files as samples
                            .map(|p| {
                                p.file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_else(|| p.to_string_lossy().to_string())
                            })
                            .collect();

                        let mut display = samples.join(", ");
                        if paths.len() > 5 {
                            display.push_str(&format!(" (+{})", paths.len() - 5));
                        }

                        // Truncate if too long for display
                        if display.len() > files_width {
                            format!("{}...", &display[0..files_width - 3])
                        } else {
                            display
                        }
                    };

                    // Display the tag name, truncate if needed
                    let tag_display = if tag.0.len() > tag_width {
                        format!("{}...", &tag.0[0..tag_width - 3])
                    } else {
                        tag.0.clone()
                    };

                    println!(
                        " {:<tag_width$} {:<count_width$} {:<files_width$}",
                        tag_display,
                        paths.len(),
                        file_samples,
                        tag_width = tag_width,
                        count_width = count_width,
                        files_width = files_width
                    );
                }
            }
            println!(
                "==============================================================================="
            );
            println!(" Total: {} tags", cache.tags_map.len());
            println!(
                "==============================================================================="
            );
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
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
        }
    }

    println!(
        "Tags listing completed - {} tags found",
        cache.tags_map.len()
    );
    Ok(())
}
