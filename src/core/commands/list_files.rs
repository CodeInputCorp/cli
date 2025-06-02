use crate::{
    core::{cache::sync_cache, types::OutputFormat},
    utils::error::{Error, Result},
};
use std::io::{self, Write};

/// Find and list files with their owners based on filter criteria
pub(crate) fn run(
    repo: Option<&std::path::Path>, tags: Option<&str>, owners: Option<&str>, unowned: bool,
    show_all: bool, format: &OutputFormat, cache_file: Option<&std::path::Path>,
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

            //  exclude unowned/untagged files unless show_all or unowned is specified
            let passes_ownership_requirement = if show_all || unowned {
                true
            } else {
                !file.owners.is_empty() || !file.tags.is_empty()
            };

            passes_owner_filter
                && passes_tag_filter
                && passes_unowned_filter
                && passes_ownership_requirement
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
