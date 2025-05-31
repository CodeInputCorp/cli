use crate::utils::error::{Error, Result};
use ignore::overrides::{Override, OverrideBuilder};

use std::path::{Path, PathBuf};

use super::types::{CodeownersEntry, FileEntry, Owner};

/// Find all files owned by a specific owner
pub fn find_files_for_owner(files: &[FileEntry], owner: &Owner) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|file_entry| file_entry.owners.contains(owner))
        .map(|file_entry| file_entry.path.clone())
        .collect()
}

/// Find owners for a specific file based on all parsed CODEOWNERS entries
pub fn find_owners_for_file(file_path: &Path, entries: &[CodeownersEntry]) -> Result<Vec<Owner>> {
    // file directory
    let target_dir = file_path
        .parent()
        .ok_or_else(|| Error::new("file path has no parent directory"))?;

    // CodeownersEntry candidates
    let mut candidates = Vec::new();

    for entry in entries {
        let codeowners_dir = match entry.source_file.parent() {
            Some(dir) => dir,
            None => {
                eprintln!(
                    "CODEOWNERS entry has no parent directory: {}",
                    entry.source_file.display()
                );
                continue;
            }
        };

        // Check if the CODEOWNERS directory is an ancestor of the target directory
        if !target_dir.starts_with(codeowners_dir) {
            continue;
        }

        // Calculate the depth as the number of components in the relative path from codeowners_dir to target_dir
        let rel_path = match target_dir.strip_prefix(codeowners_dir) {
            Ok(p) => p,
            Err(_) => continue, // Should not happen due to starts_with check
        };
        let depth = rel_path.components().count();

        // Check if the pattern matches the target file
        let matches = {
            let mut builder = OverrideBuilder::new(codeowners_dir);
            if let Err(e) = builder.add(&entry.pattern) {
                eprintln!(
                    "Invalid pattern '{}' in {}: {}",
                    entry.pattern,
                    entry.source_file.display(),
                    e
                );
                continue;
            }

            let over: Override = match builder.build() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!(
                        "Failed to build override for pattern '{}': {}",
                        entry.pattern, e
                    );
                    continue;
                }
            };
            over.matched(file_path, false).is_whitelist()
        };

        if matches {
            candidates.push((entry, depth));
        }
    }

    // Sort the candidates by depth, source file, and line number
    candidates.sort_by(|a, b| {
        let a_entry = a.0;
        let a_depth = a.1;
        let b_entry = b.0;
        let b_depth = b.1;

        // Primary sort by depth (ascending)
        a_depth
            .cmp(&b_depth)
            // Then by source file (to group entries from the same CODEOWNERS file)
            .then_with(|| a_entry.source_file.cmp(&b_entry.source_file))
            // Then by line number (descending) to prioritize later entries in the same file
            .then_with(|| b_entry.line_number.cmp(&a_entry.line_number))
    });

    // Extract the owners from the highest priority entry, if any
    Ok(candidates
        .first()
        .map(|(entry, _)| entry.owners.clone())
        .unwrap_or_default())
}
