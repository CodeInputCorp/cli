use crate::utils::error::{Error, Result};
use git2::{DiffFormat, DiffOptions, Repository};
use ignore::{
    Walk,
    overrides::{Override, OverrideBuilder},
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::types::{CodeownersEntry, FileEntry, Owner, OwnerType, Tag};

/// Find CODEOWNERS files recursively in the given directory and its subdirectories
pub fn find_codeowners_files<P: AsRef<Path>>(base_path: P) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "CODEOWNERS")
                    .unwrap_or(false)
            {
                result.push(path);
            } else if path.is_dir() {
                result.extend(find_codeowners_files(path)?);
            }
        }
    }

    Ok(result)
}

/// Find all files in the given directory and its subdirectories
pub fn find_files<P: AsRef<Path>>(base_path: P) -> Result<Vec<PathBuf>> {
    let result = Walk::new(base_path)
        .filter_map(|entry| entry.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.clone().file_name().to_str().unwrap() != "CODEOWNERS")
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    Ok(result)
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

/// Find tags for a specific file based on all parsed CODEOWNERS entries
pub fn find_tags_for_file(file_path: &Path, entries: &[CodeownersEntry]) -> Result<Vec<Tag>> {
    let target_dir = file_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file path has no parent directory",
        )
    })?;

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

    // Extract the tags from the highest priority entry, if any
    Ok(candidates
        .first()
        .map(|(entry, _)| entry.tags.clone())
        .unwrap_or_default())
}

/// Find all files owned by a specific owner
pub fn find_files_for_owner(files: &[FileEntry], owner: &Owner) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|file_entry| file_entry.owners.contains(owner))
        .map(|file_entry| file_entry.path.clone())
        .collect()
}

/// Find all files tagged with a specific tag
pub fn find_files_for_tag(files: &[FileEntry], tag: &Tag) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|file_entry| file_entry.tags.contains(tag))
        .map(|file_entry| file_entry.path.clone())
        .collect()
}

/// Collect all unique owners from CODEOWNERS entries
pub fn collect_owners(entries: &[CodeownersEntry]) -> Vec<Owner> {
    let mut owners = std::collections::HashSet::new();

    for entry in entries {
        for owner in &entry.owners {
            owners.insert(owner.clone());
        }
    }

    owners.into_iter().collect()
}

/// Collect all unique tags from CODEOWNERS entries
pub fn collect_tags(entries: &[CodeownersEntry]) -> Vec<Tag> {
    let mut tags = std::collections::HashSet::new();

    for entry in entries {
        for tag in &entry.tags {
            tags.insert(tag.clone());
        }
    }

    tags.into_iter().collect()
}

pub fn get_repo_hash(repo_path: &Path) -> Result<[u8; 32]> {
    let repo = Repository::open(repo_path)
        .map_err(|e| Error::with_source("Failed to open repo", Box::new(e)))?;

    // 1. Get HEAD commit hash (or zeros if unborn)
    let head_oid = repo
        .head()
        .and_then(|r| r.resolve())
        .and_then(|r| Ok(r.target()))
        .unwrap_or(None);

    // 2. Get index/staging area tree hash
    let mut index = repo
        .index()
        .map_err(|e| Error::with_source("Failed to get index", Box::new(e)))?;

    let index_tree = index
        .write_tree()
        .map_err(|e| Error::with_source("Failed to write index tree", Box::new(e)))?;

    // 3. Calculate hash of unstaged changes
    // TODO: this doesn't work and also we need to exclude .codeowners.cache file
    // otherwise the hash will change every time we parse the repo
    let unstaged_hash = {
        let diff = repo
            .diff_index_to_workdir(None, Some(DiffOptions::new().include_untracked(true)))
            .map_err(|e| Error::with_source("Failed to get diff", Box::new(e)))?;

        let mut hasher = Sha256::new();
        diff.print(DiffFormat::Patch, |_, _, line| {
            hasher.update(line.content());
            true
        })
        .map_err(|e| Error::with_source("Failed to print diff", Box::new(e)))?;
        hasher.finalize()
    };

    // 4. Combine all components into final hash
    let mut hasher = Sha256::new();
    hasher.update(head_oid.unwrap_or(git2::Oid::zero()).as_bytes());
    hasher.update(index_tree.as_bytes());
    hasher.update(&unstaged_hash);

    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_find_codeowners_files() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path();

        // Create test directory structure
        let sub_dir = base_path.join("subdir");
        let nested_dir = sub_dir.join("nested");
        fs::create_dir_all(&nested_dir)?;

        // Create CODEOWNERS files in different locations
        File::create(base_path.join("CODEOWNERS"))?;
        File::create(nested_dir.join("CODEOWNERS"))?;

        // Create some other files to verify we don't pick them up
        File::create(base_path.join("codeowners"))?; // wrong case
        File::create(sub_dir.join("not_codeowners"))?;

        // Find all CODEOWNERS files
        let found_files = find_codeowners_files(base_path)?;

        // Verify results
        assert_eq!(found_files.len(), 2);
        assert!(
            found_files
                .iter()
                .any(|p| p == &base_path.join("CODEOWNERS"))
        );
        assert!(
            found_files
                .iter()
                .any(|p| p == &nested_dir.join("CODEOWNERS"))
        );

        Ok(())
    }

    #[test]
    fn test_find_codeowners_files_empty_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let found_files = find_codeowners_files(temp_dir.path())?;
        assert!(found_files.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_codeowners_files_nonexistent_dir() -> Result<()> {
        let nonexistent_dir = PathBuf::from("/nonexistent/directory");
        let found_files = find_codeowners_files(nonexistent_dir)?;
        assert!(found_files.is_empty());
        Ok(())
    }
}
