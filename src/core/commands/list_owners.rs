use crate::{
    core::{cache::sync_cache, types::OutputFormat},
    utils::error::{Error, Result},
};
use std::io::{self, Write};

/// Display aggregated owner statistics and associations
pub(crate) fn run(
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
