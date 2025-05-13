use std::path::{Path, PathBuf};
use utils::error::{Error, Result};

use crate::types::{CodeownersEntry, Owner, OwnerType};

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

/// Parse CODEOWNERS
pub fn parse_codeowners(content: &str, source_path: &Path) -> Result<Vec<CodeownersEntry>> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| parse_line(line, line_num + 1, source_path).transpose())
        .collect()
}

/// Parse a line of CODEOWNERS
fn parse_line(line: &str, line_num: usize, source_path: &Path) -> Result<Option<CodeownersEntry>> {
    let clean_line = line.split('#').next().unwrap_or("").trim();
    if clean_line.is_empty() {
        return Ok(None);
    }

    let mut parts = clean_line.split_whitespace();
    let pattern = parts
        .next()
        .ok_or_else(|| Error::new("Missing pattern"))?
        .to_string();

    let (owners, tags) = parts.fold((Vec::new(), Vec::new()), |(mut owners, mut tags), part| {
        if part.starts_with('@') {
            let owner = parse_owner(part).unwrap();
            owners.push(owner);
        } else if part.starts_with('[') && part.ends_with(']') {
            tags.extend(
                part[1..part.len() - 1]
                    .split(',')
                    .map(|t| t.trim().to_string()),
            );
        } else {
            let owner = parse_owner(part).unwrap();
            owners.push(owner);
        }
        (owners, tags)
    });

    if owners.is_empty() {
        return Ok(None);
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
    let (identifier, owner_type) = if owner_str.contains('@') {
        (owner_str.to_string(), OwnerType::Email)
    } else if owner_str.starts_with('@') {
        let parts: Vec<&str> = owner_str[1..].split('/').collect();
        if parts.len() == 2 {
            (owner_str.to_string(), OwnerType::Team)
        } else {
            (owner_str.to_string(), OwnerType::User)
        }
    } else {
        (owner_str.to_string(), OwnerType::Unknown)
    };

    Ok(Owner {
        identifier,
        owner_type,
    })
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
