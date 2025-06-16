use crate::utils::error::{Error, Result};
use ignore::overrides::{Override, OverrideBuilder};

use std::path::{Path, PathBuf};

use super::types::{CodeownersEntry, FileEntry, Owner};

/// Find all files owned by a specific owner
pub fn find_files_for_owner(files: &[FileEntry], owner: &Owner) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|file_entry| {
            // Use any() with early termination instead of contains()
            if file_entry.owners.iter().any(|o| o == owner) {
                Some(file_entry.path.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Find owners for a specific file based on all parsed CODEOWNERS entries
pub fn find_owners_for_file<'a>(
    file_path: &'a Path, entries: &'a [CodeownersEntry],
) -> Result<Vec<Owner>> {
    // file directory
    let target_dir = file_path
        .parent()
        .ok_or_else(|| Error::new("file path has no parent directory"))?;

    // CodeownersEntry candidates
    let mut candidates: Vec<_> = entries
        .iter()
        .filter_map(|entry| {
            let codeowners_dir = match entry.source_file.parent() {
                Some(dir) => dir,
                None => {
                    eprintln!(
                        "CODEOWNERS entry has no parent directory: {}",
                        entry.source_file.display()
                    );
                    return None;
                }
            };

            // Check if the CODEOWNERS directory is an ancestor of the target directory
            if !target_dir.starts_with(codeowners_dir) {
                return None;
            }

            // Calculate the depth as the number of components in the relative path from codeowners_dir to target_dir
            let rel_path = match target_dir.strip_prefix(codeowners_dir) {
                Ok(p) => p,
                Err(_) => return None, // Should not happen due to starts_with check
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
                    return None;
                }

                let over: Override = match builder.build() {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!(
                            "Failed to build override for pattern '{}': {}",
                            entry.pattern, e
                        );
                        return None;
                    }
                };
                over.matched(file_path, false).is_whitelist()
            };

            if matches { Some((entry, depth)) } else { None }
        })
        .collect();

    // Sort the candidates by depth, source file, and line number
    candidates.sort_unstable_by(|a, b| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Owner, OwnerType, Tag};
    use std::path::PathBuf;

    fn create_test_owner(identifier: &str, owner_type: OwnerType) -> Owner {
        Owner {
            identifier: identifier.to_string(),
            owner_type,
        }
    }

    fn create_test_file_entry(path: &str, owners: Vec<Owner>) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            owners,
            tags: vec![],
        }
    }

    fn create_test_codeowners_entry(
        source_file: &str, line_number: usize, pattern: &str, owners: Vec<Owner>,
    ) -> CodeownersEntry {
        CodeownersEntry {
            source_file: PathBuf::from(source_file),
            line_number,
            pattern: pattern.to_string(),
            owners,
            tags: vec![],
        }
    }

    #[test]
    fn test_find_files_for_owner_empty_files() {
        let files: Vec<FileEntry> = vec![];
        let owner = create_test_owner("@user1", OwnerType::User);
        let result = find_files_for_owner(&files, &owner);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_files_for_owner_no_matches() {
        let files = vec![
            create_test_file_entry(
                "src/main.rs",
                vec![create_test_owner("@user2", OwnerType::User)],
            ),
            create_test_file_entry(
                "docs/README.md",
                vec![create_test_owner("@team1", OwnerType::Team)],
            ),
        ];
        let owner = create_test_owner("@user1", OwnerType::User);
        let result = find_files_for_owner(&files, &owner);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_files_for_owner_single_match() {
        let target_owner = create_test_owner("@user1", OwnerType::User);
        let files = vec![
            create_test_file_entry("src/main.rs", vec![target_owner.clone()]),
            create_test_file_entry(
                "docs/README.md",
                vec![create_test_owner("@team1", OwnerType::Team)],
            ),
        ];
        let result = find_files_for_owner(&files, &target_owner);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_find_files_for_owner_multiple_matches() {
        let target_owner = create_test_owner("@user1", OwnerType::User);
        let files = vec![
            create_test_file_entry("src/main.rs", vec![target_owner.clone()]),
            create_test_file_entry(
                "src/lib.rs",
                vec![
                    target_owner.clone(),
                    create_test_owner("@user2", OwnerType::User),
                ],
            ),
            create_test_file_entry(
                "docs/README.md",
                vec![create_test_owner("@team1", OwnerType::Team)],
            ),
            create_test_file_entry("tests/integration.rs", vec![target_owner.clone()]),
        ];
        let result = find_files_for_owner(&files, &target_owner);
        assert_eq!(result.len(), 3);
        let expected_paths: Vec<PathBuf> = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("tests/integration.rs"),
        ];
        for path in expected_paths {
            assert!(result.contains(&path));
        }
    }

    #[test]
    fn test_find_files_for_owner_different_owner_types() {
        let user_owner = create_test_owner("user1", OwnerType::User);
        let team_owner = create_test_owner("user1", OwnerType::Team); // Same identifier, different type

        let files = vec![
            create_test_file_entry("src/main.rs", vec![user_owner.clone()]),
            create_test_file_entry("src/lib.rs", vec![team_owner.clone()]),
        ];

        let user_result = find_files_for_owner(&files, &user_owner);
        assert_eq!(user_result.len(), 1);
        assert_eq!(user_result[0], PathBuf::from("src/main.rs"));

        let team_result = find_files_for_owner(&files, &team_owner);
        assert_eq!(team_result.len(), 1);
        assert_eq!(team_result[0], PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn test_find_owners_for_file_no_parent() {
        let entries = vec![];
        let file_path = Path::new("/");
        let result = find_owners_for_file(file_path, &entries);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_owners_for_file_no_entries() {
        let entries = vec![];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_owners_for_file_no_matches() {
        let entries = vec![create_test_codeowners_entry(
            "/other/CODEOWNERS",
            1,
            "*.py",
            vec![create_test_owner("@python-team", OwnerType::Team)],
        )];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_owners_for_file_simple_match() {
        let expected_owner = create_test_owner("@rust-team", OwnerType::Team);
        let entries = vec![create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![expected_owner.clone()],
        )];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], expected_owner);
    }

    #[test]
    fn test_find_owners_for_file_directory_hierarchy() {
        let root_owner = create_test_owner("@root-team", OwnerType::Team);
        let src_owner = create_test_owner("@src-team", OwnerType::Team);

        let entries = vec![
            create_test_codeowners_entry("/project/CODEOWNERS", 1, "*", vec![root_owner.clone()]),
            create_test_codeowners_entry(
                "/project/src/CODEOWNERS",
                1,
                "*.rs",
                vec![src_owner.clone()],
            ),
        ];

        // File in src should match the more specific src/CODEOWNERS
        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], src_owner);
    }

    #[test]
    fn test_find_owners_for_file_line_number_priority() {
        let general_owner = create_test_owner("@general-team", OwnerType::Team);
        let specific_owner = create_test_owner("@specific-team", OwnerType::Team);

        let entries = vec![
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                1,
                "*",
                vec![general_owner.clone()],
            ),
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                10,
                "src/*.rs",
                vec![specific_owner.clone()],
            ),
        ];

        // Later entry (higher line number) should take precedence
        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], specific_owner);
    }

    #[test]
    fn test_find_owners_for_file_multiple_owners() {
        let owner1 = create_test_owner("@team1", OwnerType::Team);
        let owner2 = create_test_owner("@user1", OwnerType::User);

        let entries = vec![create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![owner1.clone(), owner2.clone()],
        )];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&owner1));
        assert!(result.contains(&owner2));
    }

    #[test]
    fn test_find_owners_for_file_glob_patterns() {
        let docs_owner = create_test_owner("@docs-team", OwnerType::Team);
        let rust_owner = create_test_owner("@rust-team", OwnerType::Team);

        let entries = vec![
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                1,
                "docs/**",
                vec![docs_owner.clone()],
            ),
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                2,
                "**/*.rs",
                vec![rust_owner.clone()],
            ),
        ];

        // Test docs file
        let docs_file = Path::new("/project/docs/api/README.md");
        let result = find_owners_for_file(docs_file, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], docs_owner);

        // Test rust file
        let rust_file = Path::new("/project/src/lib.rs");
        let result = find_owners_for_file(rust_file, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], rust_owner);
    }

    #[test]
    fn test_find_owners_for_file_invalid_pattern() {
        let entries = vec![
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                1,
                "[invalid", // Invalid glob pattern
                vec![create_test_owner("@team", OwnerType::Team)],
            ),
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                2,
                "*.rs",
                vec![create_test_owner("@rust-team", OwnerType::Team)],
            ),
        ];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_for_file(file_path, &entries).unwrap();
        // Should match the valid pattern and skip the invalid one
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].identifier, "@rust-team");
    }
}
