use ignore::{
    Walk,
    overrides::{Override, OverrideBuilder},
};
use std::path::{Path, PathBuf};
use utils::error::{Error, Result};

use crate::types::{CodeownersEntry, FileEntry, Owner, OwnerType, Tag};

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

/// Parse CODEOWNERS
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
