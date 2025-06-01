use crate::utils::error::{Error, Result};
use ignore::overrides::{Override, OverrideBuilder};

use std::path::{Path, PathBuf};

use super::{
    smart_iter::SmartIter,
    types::{CodeownersEntry, FileEntry, Tag},
};

/// Find all files tagged with a specific tag
pub fn find_files_for_tag(files: &[FileEntry], tag: &Tag) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|file_entry| {
            if file_entry.tags.contains(tag) {
                Some(file_entry.path.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Find tags for a specific file based on all parsed CODEOWNERS entries
pub fn find_tags_for_file(file_path: &Path, entries: &[CodeownersEntry]) -> Result<Vec<Tag>> {
    let target_dir = file_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file path has no parent directory",
        )
    })?;

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

    // Extract the tags from the highest priority entry, if any
    Ok(candidates
        .first()
        .map(|(entry, _)| entry.tags.clone())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Owner, OwnerType, Tag};
    use std::path::PathBuf;

    fn create_test_tag(name: &str) -> Tag {
        Tag(name.to_string())
    }

    fn create_test_owner(identifier: &str, owner_type: OwnerType) -> Owner {
        Owner {
            identifier: identifier.to_string(),
            owner_type,
        }
    }

    fn create_test_file_entry(path: &str, tags: Vec<Tag>) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            owners: vec![],
            tags,
        }
    }

    fn create_test_codeowners_entry(
        source_file: &str, line_number: usize, pattern: &str, tags: Vec<Tag>,
    ) -> CodeownersEntry {
        CodeownersEntry {
            source_file: PathBuf::from(source_file),
            line_number,
            pattern: pattern.to_string(),
            owners: vec![],
            tags,
        }
    }

    #[test]
    fn test_find_files_for_tag_empty_files() {
        let files: Vec<FileEntry> = vec![];
        let tag = create_test_tag("frontend");
        let result = find_files_for_tag(&files, &tag);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_files_for_tag_no_matches() {
        let files = vec![
            create_test_file_entry("src/main.rs", vec![create_test_tag("backend")]),
            create_test_file_entry("docs/README.md", vec![create_test_tag("documentation")]),
        ];
        let tag = create_test_tag("frontend");
        let result = find_files_for_tag(&files, &tag);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_files_for_tag_single_match() {
        let target_tag = create_test_tag("frontend");
        let files = vec![
            create_test_file_entry("src/main.rs", vec![create_test_tag("backend")]),
            create_test_file_entry("web/index.html", vec![target_tag.clone()]),
            create_test_file_entry("docs/README.md", vec![create_test_tag("documentation")]),
        ];
        let result = find_files_for_tag(&files, &target_tag);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], PathBuf::from("web/index.html"));
    }

    #[test]
    fn test_find_files_for_tag_multiple_matches() {
        let target_tag = create_test_tag("api");
        let files = vec![
            create_test_file_entry("src/api/mod.rs", vec![target_tag.clone()]),
            create_test_file_entry(
                "src/api/users.rs",
                vec![target_tag.clone(), create_test_tag("users")],
            ),
            create_test_file_entry("src/main.rs", vec![create_test_tag("backend")]),
            create_test_file_entry("tests/api_tests.rs", vec![target_tag.clone()]),
        ];
        let result = find_files_for_tag(&files, &target_tag);
        assert_eq!(result.len(), 3);
        let expected_paths: Vec<PathBuf> = vec![
            PathBuf::from("src/api/mod.rs"),
            PathBuf::from("src/api/users.rs"),
            PathBuf::from("tests/api_tests.rs"),
        ];
        for path in expected_paths {
            assert!(result.contains(&path));
        }
    }

    #[test]
    fn test_find_files_for_tag_multiple_tags_per_file() {
        let api_tag = create_test_tag("api");
        let users_tag = create_test_tag("users");
        let admin_tag = create_test_tag("admin");

        let files = vec![
            create_test_file_entry("src/api/users.rs", vec![api_tag.clone(), users_tag.clone()]),
            create_test_file_entry("src/api/admin.rs", vec![api_tag.clone(), admin_tag.clone()]),
            create_test_file_entry("src/main.rs", vec![create_test_tag("backend")]),
        ];

        // Test finding files for api tag
        let api_result = find_files_for_tag(&files, &api_tag);
        assert_eq!(api_result.len(), 2);
        assert!(api_result.contains(&PathBuf::from("src/api/users.rs")));
        assert!(api_result.contains(&PathBuf::from("src/api/admin.rs")));

        // Test finding files for users tag
        let users_result = find_files_for_tag(&files, &users_tag);
        assert_eq!(users_result.len(), 1);
        assert_eq!(users_result[0], PathBuf::from("src/api/users.rs"));

        // Test finding files for admin tag
        let admin_result = find_files_for_tag(&files, &admin_tag);
        assert_eq!(admin_result.len(), 1);
        assert_eq!(admin_result[0], PathBuf::from("src/api/admin.rs"));
    }

    #[test]
    fn test_find_tags_for_file_no_parent() {
        let entries = vec![];
        let file_path = Path::new("/");
        let result = find_tags_for_file(file_path, &entries);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_tags_for_file_no_entries() {
        let entries = vec![];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_tags_for_file_no_matches() {
        let entries = vec![create_test_codeowners_entry(
            "/other/CODEOWNERS",
            1,
            "*.py",
            vec![create_test_tag("python")],
        )];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_tags_for_file_simple_match() {
        let expected_tag = create_test_tag("rust");
        let entries = vec![create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![expected_tag.clone()],
        )];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], expected_tag);
    }

    #[test]
    fn test_find_tags_for_file_directory_hierarchy() {
        let root_tag = create_test_tag("root");
        let src_tag = create_test_tag("source");

        let entries = vec![
            create_test_codeowners_entry("/project/CODEOWNERS", 1, "*", vec![root_tag.clone()]),
            create_test_codeowners_entry(
                "/project/src/CODEOWNERS",
                1,
                "*.rs",
                vec![src_tag.clone()],
            ),
        ];

        // File in src should match the more specific src/CODEOWNERS
        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], src_tag);
    }

    #[test]
    fn test_find_tags_for_file_line_number_priority() {
        let general_tag = create_test_tag("general");
        let specific_tag = create_test_tag("specific");

        let entries = vec![
            create_test_codeowners_entry("/project/CODEOWNERS", 1, "*", vec![general_tag.clone()]),
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                10,
                "src/*.rs",
                vec![specific_tag.clone()],
            ),
        ];

        // Later entry (higher line number) should take precedence
        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], specific_tag);
    }

    #[test]
    fn test_find_tags_for_file_multiple_tags() {
        let tag1 = create_test_tag("api");
        let tag2 = create_test_tag("core");

        let entries = vec![create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![tag1.clone(), tag2.clone()],
        )];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&tag1));
        assert!(result.contains(&tag2));
    }

    #[test]
    fn test_find_tags_for_file_glob_patterns() {
        let docs_tag = create_test_tag("documentation");
        let rust_tag = create_test_tag("rust");

        let entries = vec![
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                1,
                "docs/**",
                vec![docs_tag.clone()],
            ),
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                2,
                "**/*.rs",
                vec![rust_tag.clone()],
            ),
        ];

        // Test docs file
        let docs_file = Path::new("/project/docs/api/README.md");
        let result = find_tags_for_file(docs_file, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], docs_tag);

        // Test rust file
        let rust_file = Path::new("/project/src/lib.rs");
        let result = find_tags_for_file(rust_file, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], rust_tag);
    }

    #[test]
    fn test_find_tags_for_file_invalid_pattern() {
        let entries = vec![
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                1,
                "[invalid", // Invalid glob pattern
                vec![create_test_tag("invalid")],
            ),
            create_test_codeowners_entry(
                "/project/CODEOWNERS",
                2,
                "*.rs",
                vec![create_test_tag("rust")],
            ),
        ];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        // Should match the valid pattern and skip the invalid one
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "rust");
    }

    #[test]
    fn test_find_tags_for_file_complex_hierarchy() {
        let root_tag = create_test_tag("project");
        let backend_tag = create_test_tag("backend");
        let api_tag = create_test_tag("api");

        let entries = vec![
            create_test_codeowners_entry("/project/CODEOWNERS", 1, "*", vec![root_tag.clone()]),
            create_test_codeowners_entry(
                "/project/src/CODEOWNERS",
                1,
                "**",
                vec![backend_tag.clone()],
            ),
            create_test_codeowners_entry(
                "/project/src/api/CODEOWNERS",
                1,
                "*.rs",
                vec![api_tag.clone()],
            ),
        ];

        // File deep in hierarchy should match the most specific entry
        let file_path = Path::new("/project/src/api/users.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], api_tag);

        // File in src but not api should match backend tag
        let file_path = Path::new("/project/src/main.rs");
        let result = find_tags_for_file(file_path, &entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], backend_tag);
    }
}
