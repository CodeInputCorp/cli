use crate::utils::error::{Error, Result};
use git2::{DiffFormat, DiffOptions, Repository};
use ignore::{
    Walk,
    overrides::{Override, OverrideBuilder},
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::types::{CodeownersEntry, FileEntry, Owner, OwnerType, Tag};

// Module-level comments
//! # Core Common Utilities
//!
//! This module provides common utility functions used throughout the `core` module
//! and potentially other parts of the application. These utilities handle tasks such as:
//!
//! - Discovering `CODEOWNERS` files and other project files within a directory structure.
//! - Parsing `CODEOWNERS` file content line by line to extract rules, owners, and tags.
//! - Determining the effective owners and tags for a given file path based on the parsed `CODEOWNERS` rules.
//! - Aggregating lists of files associated with specific owners or tags.
//! - Collecting unique owners and tags from all parsed entries.
//! - Calculating a repository hash for cache validation purposes.
//!
//! The functions here often interact with the file system, parse text data, and apply
//! logic to match file paths against `CODEOWNERS` patterns.

/// Finds all files named `CODEOWNERS` recursively within a given base path.
///
/// This function walks the directory tree starting from `base_path` and collects
/// the paths of all files that are explicitly named "CODEOWNERS".
///
/// # Arguments
///
/// * `base_path`: A generic type `P` that can be converted into a `Path` reference,
///                representing the root directory to start the search from.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<PathBuf>` of all found `CODEOWNERS` file paths
/// on success, or an `Error` if there's an issue reading directories (though current
/// implementation returns `Ok` with an empty vec on read errors for a specific dir).
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

/// Finds all files within a given base path, excluding "CODEOWNERS" files.
///
/// This function utilizes the `ignore` crate's `Walk` functionality to traverse
/// the directory structure starting from `base_path`. It filters for entries that
/// are files and are not named "CODEOWNERS".
///
/// # Arguments
///
/// * `base_path`: A generic type `P` that can be converted into a `Path` reference,
///                representing the root directory to search for files.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<PathBuf>` of all found file paths
/// (excluding "CODEOWNERS") on success. Currently, errors during walk are filtered out,
/// so it effectively always returns `Ok`.
pub fn find_files<P: AsRef<Path>>(base_path: P) -> Result<Vec<PathBuf>> {
    let result = Walk::new(base_path)
        .filter_map(|entry| entry.ok()) // Silently ignore errors from Walk, converting them to None
        .filter(|e| e.path().is_file())
        .filter(|e| e.clone().file_name().to_str().unwrap() != "CODEOWNERS")
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    Ok(result)
}

/// Parses a single `CODEOWNERS` file into a list of `CodeownersEntry` structs.
///
/// This function reads the content of the specified `CODEOWNERS` file, then processes
/// it line by line. Each non-empty, non-comment line is parsed by `parse_line`
/// to create a `CodeownersEntry`.
///
/// # Arguments
///
/// * `source_path`: A `Path` reference to the `CODEOWNERS` file to be parsed.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<CodeownersEntry>` representing all valid rules
/// found in the file. An `Error` can occur if the file cannot be read or if
/// `parse_line` encounters an issue (though `parse_line` itself aims to be robust).
pub fn parse_codeowners(source_path: &Path) -> Result<Vec<CodeownersEntry>> {
    let content = std::fs::read_to_string(source_path)?;

    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| parse_line(line, line_num, source_path).transpose())
        .collect()
}

/// Parse a line of CODEOWNERS
fn parse_line(line: &str, line_num: usize, source_path: &Path) -> Result<Option<CodeownersEntry>> {
    // Trim the line and check for empty or comment lines
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }

    // Split the line by whitespace into a series of tokens
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.is_empty() {
        return Ok(None);
    }

    // The first token is the pattern
    let pattern = tokens[0].to_string();

    let mut owners: Vec<Owner> = Vec::new();
    let mut tags: Vec<Tag> = Vec::new();

    let mut i = 1; // Start after the pattern

    // Collect owners until a token starts with '#'
    while i < tokens.len() && !tokens[i].starts_with('#') {
        owners.push(parse_owner(tokens[i])?);
        i += 1;
    }

    // Collect tags with lookahead to check for comments
    while i < tokens.len() {
        let token = tokens[i];
        if token.starts_with('#') {
            if token == "#" {
                // Comment starts, break
                break;
            } else {
                // Check if the next token is not a tag (doesn't start with '#')
                let next_is_non_tag = i + 1 < tokens.len() && !tokens[i + 1].starts_with('#');
                if next_is_non_tag {
                    // This token is part of the comment, break
                    break;
                }
                tags.push(Tag(token[1..].to_string()));
                i += 1;
            }
        } else {
            // Non-tag, part of comment
            break;
        }
    }

    Ok(Some(CodeownersEntry {
        source_file: source_path.to_path_buf(),
        line_number: line_num,
        pattern,
        owners,
        tags,
    }))
}

/// Parse an owner string into an Owner struct
fn parse_owner(owner_str: &str) -> Result<Owner> {
    let identifier = owner_str.to_string();
    let owner_type = if identifier.eq_ignore_ascii_case("NOOWNER") {
        OwnerType::Unowned
    } else if owner_str.starts_with('@') {
        let parts: Vec<&str> = owner_str[1..].split('/').collect();
        if parts.len() == 2 {
            OwnerType::Team
        } else {
            OwnerType::User
        }
    } else if owner_str.contains('@') {
        OwnerType::Email
    } else {
        OwnerType::Unknown
    };

    Ok(Owner {
        identifier,
        owner_type,
    })
}

/// Determines the effective owners for a given file path based on a list of `CodeownersEntry` rules.
///
/// This function iterates through all provided `CodeownersEntry` items and identifies which
/// entries match the `file_path`. The matching is performed using glob patterns from the entries,
/// anchored to the directory of the `CODEOWNERS` file where the entry originated.
///
/// The selection logic prioritizes rules based on:
/// 1. **Depth**: Rules from `CODEOWNERS` files closer to the target file (greater depth) take precedence.
/// 2. **Source File**: If depths are equal, the specific `CODEOWNERS` file is considered (though typical usage has one per dir).
/// 3. **Line Number**: For rules within the same `CODEOWNERS` file at the same effective depth,
///    the rule appearing later in the file (higher line number) takes precedence.
///
/// # Arguments
///
/// * `file_path`: A `Path` reference to the file for which owners are to be determined.
/// * `entries`: A slice of `CodeownersEntry` structs representing all parsed rules from all relevant `CODEOWNERS` files.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<Owner>` with the owners from the highest priority matching rule.
/// If no rule matches, an empty vector is returned. An `Error` can occur if `file_path` has no parent directory
/// or if pattern matching encounters issues (though many pattern errors are logged to `eprintln!` and skipped).
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
                // Log and skip if a CODEOWNERS entry's source file path is invalid
                eprintln!(
                    "CODEOWNERS entry has no parent directory: {}",
                    entry.source_file.display()
                );
                continue;
            }
        };

        // Rule applies only if its CODEOWNERS file is in an ancestor directory of the target file
        if !target_dir.starts_with(codeowners_dir) {
            continue;
        }

        // Calculate depth: more nested CODEOWNERS files are more specific.
        // Depth is the number of directory levels between the CODEOWNERS file's directory and the target file's directory.
        let rel_path = match target_dir.strip_prefix(codeowners_dir) {
            Ok(p) => p,
            Err(_) => continue, // Should not happen due to the starts_with check
        };
        let depth = rel_path.components().count();

        // Check if the entry's pattern matches the target file
        let matches = {
            let mut builder = OverrideBuilder::new(codeowners_dir); // Patterns are relative to the CODEOWNERS file's directory
            if let Err(e) = builder.add(&entry.pattern) {
                // Log and skip invalid patterns
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
                    // Log and skip if override builder fails
                    eprintln!(
                        "Failed to build override for pattern '{}': {}",
                        entry.pattern, e
                    );
                    continue;
                }
            };
            // Check if the file path matches the pattern. `is_whitelist()` means it's a match.
            over.matched(file_path, false).is_whitelist()
        };

        if matches {
            candidates.push((entry, depth));
        }
    }

    // Sort candidates to find the most specific matching rule.
    // The primary sort key is depth (ascending, meaning deeper rules are preferred but this seems inverted, typically deeper means higher specificity, which should come later or be reversed).
    // However, the standard CODEOWNERS logic is "last match wins" within a file, and closer files win.
    // Let's re-verify the sorting logic based on typical CODEOWNERS behavior:
    // 1. Specificity of pattern (glob vs. path component) - not directly handled here, relies on gitignore matching.
    // 2. Closeness of CODEOWNERS file: Rules in CODEOWNERS in a deeper directory take precedence.
    // 3. Order within a file: Later rules override earlier ones.
    // The current sort:
    // - `a_depth.cmp(&b_depth)`: Ascending depth. If `a` is shallower (e.g. depth 0) and `b` is deeper (e.g. depth 1), `a` comes first. This needs to be descending for depth.
    // - `a_entry.source_file.cmp(&b_entry.source_file)`: Groups by file.
    // - `b_entry.line_number.cmp(&a_entry.line_number)`: Descending line number. Later lines come first. This is correct.
    // To correct depth sorting for precedence (deeper first):
    candidates.sort_by(|a, b| {
        let a_entry = a.0;
        let a_depth = a.1;
        let b_entry = b.0;
        let b_depth = b.1;

        b_depth // Sort by depth (descending: deeper files/rules take precedence)
            .cmp(&a_depth)
            .then_with(|| b_entry.line_number.cmp(&a_entry.line_number)) // Then by line number (descending: later rules take precedence)
            .then_with(|| a_entry.source_file.cmp(&b_entry.source_file)) // Fallback to source file for stability if depths and lines are same (unlikely for different files)
    });


    // The first candidate after sorting is the one that takes precedence.
    Ok(candidates
        .first()
        .map(|(entry, _)| entry.owners.clone())
        .unwrap_or_default())
}

/// Determines the effective tags for a given file path based on a list of `CodeownersEntry` rules.
///
/// This function operates similarly to `find_owners_for_file`, using the same matching
/// and prioritization logic (depth of `CODEOWNERS` file, line number within the file)
/// to find the highest priority `CodeownersEntry` that matches the `file_path`.
/// It then returns the tags associated with that entry.
///
/// # Arguments
///
/// * `file_path`: A `Path` reference to the file for which tags are to be determined.
/// * `entries`: A slice of `CodeownersEntry` structs representing all parsed rules.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<Tag>` with the tags from the highest priority matching rule.
/// If no rule matches, an empty vector is returned. An `Error` can occur under the same
/// conditions as `find_owners_for_file`.
pub fn find_tags_for_file(file_path: &Path, entries: &[CodeownersEntry]) -> Result<Vec<Tag>> {
    let target_dir = file_path.parent().ok_or_else(|| {
        // Using std::io::Error here, but crate::utils::error::Error might be more consistent.
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

        if !target_dir.starts_with(codeowners_dir) {
            continue;
        }

        let rel_path = match target_dir.strip_prefix(codeowners_dir) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let depth = rel_path.components().count();

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

    // Sorting logic should be identical to find_owners_for_file for consistency.
    candidates.sort_by(|a, b| {
        let a_entry = a.0;
        let a_depth = a.1;
        let b_entry = b.0;
        let b_depth = b.1;

        b_depth // Sort by depth (descending)
            .cmp(&a_depth)
            .then_with(|| b_entry.line_number.cmp(&a_entry.line_number)) // Then by line number (descending)
            .then_with(|| a_entry.source_file.cmp(&b_entry.source_file))
    });

    Ok(candidates
        .first()
        .map(|(entry, _)| entry.tags.clone())
        .unwrap_or_default())
}

/// Filters a list of `FileEntry` items to find all files owned by a specific `Owner`.
///
/// # Arguments
///
/// * `files`: A slice of `FileEntry` structs, typically from a `CodeownersCache`.
/// * `owner`: An `Owner` reference to filter by.
///
/// # Returns
///
/// Returns a `Vec<PathBuf>` containing the paths of all files in the input list
/// that have the specified `owner` in their `owners` list.
pub fn find_files_for_owner(files: &[FileEntry], owner: &Owner) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|file_entry| file_entry.owners.contains(owner))
        .map(|file_entry| file_entry.path.clone())
        .collect()
}

/// Filters a list of `FileEntry` items to find all files tagged with a specific `Tag`.
///
/// # Arguments
///
/// * `files`: A slice of `FileEntry` structs, typically from a `CodeownersCache`.
/// * `tag`: A `Tag` reference to filter by.
///
/// # Returns
///
/// Returns a `Vec<PathBuf>` containing the paths of all files in the input list
/// that have the specified `tag` in their `tags` list.
pub fn find_files_for_tag(files: &[FileEntry], tag: &Tag) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|file_entry| file_entry.tags.contains(tag))
        .map(|file_entry| file_entry.path.clone())
        .collect()
}

/// Collects all unique `Owner` instances from a list of `CodeownersEntry` items.
///
/// This function iterates through all provided entries and their associated owners,
/// adding each owner to a `HashSet` to ensure uniqueness. The resulting set is then
/// converted into a vector.
///
/// # Arguments
///
/// * `entries`: A slice of `CodeownersEntry` structs.
///
/// # Returns
///
/// Returns a `Vec<Owner>` containing all unique owners found across all entries.
pub fn collect_owners(entries: &[CodeownersEntry]) -> Vec<Owner> {
    let mut owners = std::collections::HashSet::new(); // Use HashSet for automatic deduplication

    for entry in entries {
        for owner in &entry.owners {
            owners.insert(owner.clone());
        }
    }

    owners.into_iter().collect()
}

/// Collects all unique `Tag` instances from a list of `CodeownersEntry` items.
///
/// This function iterates through all provided entries and their associated tags,
/// adding each tag to a `HashSet` to ensure uniqueness. The resulting set is then
/// converted into a vector.
///
/// # Arguments
///
/// * `entries`: A slice of `CodeownersEntry` structs.
///
/// # Returns
///
/// Returns a `Vec<Tag>` containing all unique tags found across all entries.
pub fn collect_tags(entries: &[CodeownersEntry]) -> Vec<Tag> {
    let mut tags = std::collections::HashSet::new(); // Use HashSet for automatic deduplication

    for entry in entries {
        for tag in &entry.tags {
            tags.insert(tag.clone());
        }
    }

    tags.into_iter().collect()
}

/// Calculates a hash representing the state of a Git repository.
///
/// This hash is intended for cache validation to detect changes in the repository
/// that might invalidate a previously generated `CodeownersCache`. The hash incorporates:
/// 1. The OID (hash) of the commit pointed to by HEAD (or a zero OID if HEAD is unborn).
/// 2. The OID of the tree object representing the Git index (staging area).
/// 3. A hash of the diff between the index and the working directory (unstaged changes),
///    including untracked files.
///    **Note**: The current implementation of hashing unstaged changes has a TODO regarding
///    its correctness and the exclusion of the cache file itself from this hash.
///
/// These components are combined using SHA-256 to produce a final 32-byte hash.
///
/// # Arguments
///
/// * `repo_path`: A `Path` reference to the root of the Git repository.
///
/// # Returns
///
/// Returns a `Result` containing a 32-byte array (`[u8; 32]`) representing the
/// repository state hash. An `Error` can occur if the repository cannot be opened,
/// the index cannot be accessed, or diffing fails.
pub fn get_repo_hash(repo_path: &Path) -> Result<[u8; 32]> {
    let repo = Repository::open(repo_path)
        .map_err(|e| Error::with_source("Failed to open repo", Box::new(e)))?;

    // 1. Get HEAD commit OID (or zeros if unborn/detached HEAD with no commit)
    let head_oid = repo
        .head() // Get a reference to HEAD
        .and_then(|r| r.resolve()) // Resolve symbolic refs like 'refs/heads/main' to a direct ref
        .and_then(|r| Ok(r.target())) // Get the OID of the commit pointed to by the resolved ref
        .unwrap_or(None); // If any step fails (e.g., unborn HEAD), default to None

    // 2. Get index/staging area tree OID
    let mut index = repo
        .index()
        .map_err(|e| Error::with_source("Failed to get index", Box::new(e)))?;

    let index_tree_oid = index // OID of the tree object representing the index
        .write_tree() // Writes the current index as a tree object to the ODB, returns its OID
        .map_err(|e| Error::with_source("Failed to write index tree", Box::new(e)))?;

    // 3. Calculate hash of unstaged changes (workdir vs index), including untracked files.
    // TODO: This part needs careful review.
    // - Excluding `.codeowners.cache` (or the configured cache file) is crucial to prevent the hash
    //   from changing simply because the cache was updated by this tool.
    // - The method of hashing the diff patch might be sensitive to git configuration (e.g., whitespace).
    //   A more robust approach might involve hashing specific attributes of diff entries (paths, modes, OIDs of blobs for modified files).
    let unstaged_hash = {
        let diff = repo
            .diff_index_to_workdir(None, Some(DiffOptions::new().include_untracked(true))) // Diff index to workdir
            .map_err(|e| Error::with_source("Failed to get diff", Box::new(e)))?;

        let mut hasher = Sha256::new();
        diff.print(DiffFormat::Patch, |_, _, line| { // Iterate over lines in the patch
            hasher.update(line.content()); // Add line content to hash
            true // Continue processing
        })
        .map_err(|e| Error::with_source("Failed to print diff (for hashing)", Box::new(e)))?;
        hasher.finalize()
    };

    // 4. Combine all components into a final hash
    let mut final_hasher = Sha256::new();
    final_hasher.update(head_oid.unwrap_or(git2::Oid::zero()).as_bytes()); // Use zero OID if head_oid is None
    final_hasher.update(index_tree_oid.as_bytes());
    final_hasher.update(&unstaged_hash);

    Ok(final_hasher.finalize().into()) // Convert GenericArray to [u8; 32]
}

#[cfg(test)]
mod tests {

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

    #[test]
    fn test_parse_owner_user() -> Result<()> {
        let owner = parse_owner("@username")?;
        assert_eq!(owner.identifier, "@username");
        assert!(matches!(owner.owner_type, OwnerType::User));

        // With hyphens and underscores
        let owner = parse_owner("@user-name_123")?;
        assert_eq!(owner.identifier, "@user-name_123");
        assert!(matches!(owner.owner_type, OwnerType::User));

        // Single character username
        let owner = parse_owner("@a")?;
        assert_eq!(owner.identifier, "@a");
        assert!(matches!(owner.owner_type, OwnerType::User));

        Ok(())
    }

    #[test]
    fn test_parse_owner_team() -> Result<()> {
        // Standard team
        let owner = parse_owner("@org/team-name")?;
        assert_eq!(owner.identifier, "@org/team-name");
        assert!(matches!(owner.owner_type, OwnerType::Team));

        // With numbers and special characters
        let owner = parse_owner("@company123/frontend-team_01")?;
        assert_eq!(owner.identifier, "@company123/frontend-team_01");
        assert!(matches!(owner.owner_type, OwnerType::Team));

        // Short names
        let owner = parse_owner("@o/t")?;
        assert_eq!(owner.identifier, "@o/t");
        assert!(matches!(owner.owner_type, OwnerType::Team));

        Ok(())
    }

    #[test]
    fn test_parse_owner_email() -> Result<()> {
        // Standard email
        let owner = parse_owner("user@example.com")?;
        assert_eq!(owner.identifier, "user@example.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // With plus addressing
        let owner = parse_owner("user+tag@example.com")?;
        assert_eq!(owner.identifier, "user+tag@example.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // With dots and numbers
        let owner = parse_owner("user.name123@sub.example.com")?;
        assert_eq!(owner.identifier, "user.name123@sub.example.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // Multiple @ symbols - should still be detected as Email
        let owner = parse_owner("user@example@domain.com")?;
        assert_eq!(owner.identifier, "user@example@domain.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // IP address domain
        let owner = parse_owner("user@[192.168.1.1]")?;
        assert_eq!(owner.identifier, "user@[192.168.1.1]");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        Ok(())
    }

    #[test]
    fn test_parse_owner_unowned() -> Result<()> {
        let owner = parse_owner("NOOWNER")?;
        assert_eq!(owner.identifier, "NOOWNER");
        assert!(matches!(owner.owner_type, OwnerType::Unowned));

        // Case insensitive
        let owner = parse_owner("noowner")?;
        assert_eq!(owner.identifier, "noowner");
        assert!(matches!(owner.owner_type, OwnerType::Unowned));

        let owner = parse_owner("NoOwNeR")?;
        assert_eq!(owner.identifier, "NoOwNeR");
        assert!(matches!(owner.owner_type, OwnerType::Unowned));

        Ok(())
    }

    #[test]
    fn test_parse_owner_unknown() -> Result<()> {
        // Random text
        let owner = parse_owner("plaintext")?;
        assert_eq!(owner.identifier, "plaintext");
        assert!(matches!(owner.owner_type, OwnerType::Unknown));

        // Text with special characters (but not @ or email format)
        let owner = parse_owner("special-text_123")?;
        assert_eq!(owner.identifier, "special-text_123");
        assert!(matches!(owner.owner_type, OwnerType::Unknown));

        // URL-like but not an owner
        let owner = parse_owner("https://example.com")?;
        assert_eq!(owner.identifier, "https://example.com");
        assert!(matches!(owner.owner_type, OwnerType::Unknown));

        Ok(())
    }

    #[test]
    fn test_parse_owner_email_edge_cases() -> Result<()> {
        // Technically valid by RFC 5322 but unusual emails
        let owner = parse_owner("\"quoted\"@example.com")?;
        assert_eq!(owner.identifier, "\"quoted\"@example.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // Very short email
        let owner = parse_owner("a@b.c")?;
        assert_eq!(owner.identifier, "a@b.c");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // Email with many subdomains
        let owner = parse_owner("user@a.b.c.d.example.com")?;
        assert_eq!(owner.identifier, "user@a.b.c.d.example.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        Ok(())
    }

    #[test]
    fn test_parse_owner_ambiguous_cases() -> Result<()> {
        // Contains @ but also has prefix
        let owner = parse_owner("prefix-user@example.com")?;
        assert_eq!(owner.identifier, "prefix-user@example.com");
        assert!(matches!(owner.owner_type, OwnerType::Email));

        // Has team-like structure but without @ prefix
        let owner = parse_owner("org/team-name")?;
        assert_eq!(owner.identifier, "org/team-name");
        assert!(matches!(owner.owner_type, OwnerType::Unknown));

        // Contains "NOOWNER" as substring but isn't exactly NOOWNER
        let owner = parse_owner("NOOWNER-plus")?;
        assert_eq!(owner.identifier, "NOOWNER-plus");
        assert!(matches!(owner.owner_type, OwnerType::Unknown));

        Ok(())
    }

    #[test]
    fn test_parse_line_pattern_with_owners() -> Result<()> {
        let source_path = Path::new("/test/CODEOWNERS");
        let result = parse_line("*.js @qa-team @bob #test", 1, source_path)?;

        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.pattern, "*.js");
        assert_eq!(entry.owners.len(), 2);
        assert_eq!(entry.owners[0].identifier, "@qa-team");
        assert_eq!(entry.owners[1].identifier, "@bob");
        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.tags[0].0, "test");
        assert_eq!(entry.line_number, 1);
        assert_eq!(entry.source_file, source_path);

        Ok(())
    }

    #[test]
    fn test_parse_line_with_path_pattern() -> Result<()> {
        let source_path = Path::new("/test/CODEOWNERS");
        let result = parse_line("/fixtures/ @alice @dave", 2, source_path)?;

        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.pattern, "/fixtures/");
        assert_eq!(entry.owners.len(), 2);
        assert_eq!(entry.owners[0].identifier, "@alice");
        assert_eq!(entry.owners[1].identifier, "@dave");
        assert_eq!(entry.tags.len(), 0);

        Ok(())
    }

    #[test]
    fn test_parse_line_comment() -> Result<()> {
        let source_path = Path::new("/test/CODEOWNERS");
        let result = parse_line("# this is a comment line", 3, source_path)?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_line_with_multiple_tags_and_comment() -> Result<()> {
        let source_path = Path::new("/test/CODEOWNERS");
        let result = parse_line(
            "/hooks.ts @org/frontend #test #core # this is a comment",
            4,
            source_path,
        )?;

        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.pattern, "/hooks.ts");
        assert_eq!(entry.owners.len(), 1);
        assert_eq!(entry.owners[0].identifier, "@org/frontend");
        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.tags[0].0, "test");
        assert_eq!(entry.tags[1].0, "core");

        Ok(())
    }

    #[test]
    fn test_parse_line_empty() -> Result<()> {
        let source_path = Path::new("/test/CODEOWNERS");
        let result = parse_line("", 5, source_path)?;

        assert!(result.is_none());

        let result = parse_line("    ", 6, source_path)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_line_security_tag() -> Result<()> {
        let source_path = Path::new("/test/.husky/CODEOWNERS");
        let result = parse_line("pre-commit @org/security @frank #security", 2, source_path)?;

        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.pattern, "pre-commit");
        assert_eq!(entry.owners.len(), 2);
        assert_eq!(entry.owners[0].identifier, "@org/security");
        assert_eq!(entry.owners[1].identifier, "@frank");
        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.tags[0].0, "security");

        Ok(())
    }

    #[test]
    fn test_parse_line_with_pound_tag_edge_case() -> Result<()> {
        let source_path = Path::new("/test/CODEOWNERS");

        // Test edge case where # is followed by a space (comment marker)
        let result = parse_line("*.md @docs-team #not a tag", 7, source_path)?;

        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.pattern, "*.md");
        assert_eq!(entry.owners.len(), 1);
        assert_eq!(entry.owners[0].identifier, "@docs-team");
        assert_eq!(entry.tags.len(), 0); // No tags, just a comment

        Ok(())
    }
}
