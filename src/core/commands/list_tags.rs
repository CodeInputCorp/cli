use crate::{
    core::{cache::sync_cache, types::OutputFormat},
    utils::error::{Error, Result},
};
use std::io::{self, Write};

/// Audit and analyze tag usage across CODEOWNERS files
pub(crate) fn run(
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
