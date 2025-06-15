use super::{
    inline_parser::detect_inline_codeowners,
    smart_iter::SmartIter,
    types::{CodeownersEntryMatcher, Tag},
};
use crate::utils::error::{Error, Result};
use ignore::overrides::{Override, OverrideBuilder};

use std::{path::Path, time::Instant};

use super::types::{CodeownersEntry, Owner};

/// Find both owners and tags for a specific file based on all parsed CODEOWNERS entries
pub fn find_owners_and_tags_for_file(
    file_path: &Path, entries: &[CodeownersEntryMatcher],
) -> Result<(Vec<Owner>, Vec<Tag>)> {
    // First, check for inline CODEOWNERS declaration (highest priority)
    if let Some(inline_entry) = detect_inline_codeowners(file_path)? {
        return Ok((inline_entry.owners, inline_entry.tags));
    }

    // Early return if no entries
    if entries.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let target_dir = file_path
        .parent()
        .ok_or_else(|| Error::new("file path has no parent directory"))?;

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
                entry
                    .override_matcher
                    .matched(file_path, false)
                    .is_whitelist()
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

    // Extract both owners and tags from the highest priority entry, if any
    Ok(candidates
        .first()
        .map(|(entry, _)| (entry.owners.clone(), entry.tags.clone()))
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Owner, OwnerType, Tag};
    use ignore::overrides::OverrideBuilder;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_owner(identifier: &str, owner_type: OwnerType) -> Owner {
        Owner {
            identifier: identifier.to_string(),
            owner_type,
        }
    }

    fn create_test_tag(name: &str) -> Tag {
        Tag(name.to_string())
    }

    fn create_test_codeowners_entry_matcher(
        source_file: &str, line_number: usize, pattern: &str, owners: Vec<Owner>, tags: Vec<Tag>,
    ) -> CodeownersEntryMatcher {
        let source_path = PathBuf::from(source_file);
        let codeowners_dir = source_path.parent().unwrap();

        let mut builder = OverrideBuilder::new(codeowners_dir);
        builder.add(pattern).unwrap();
        let override_matcher = builder.build().unwrap();

        CodeownersEntryMatcher {
            source_file: source_path,
            line_number,
            pattern: pattern.to_string(),
            owners,
            tags,
            override_matcher,
        }
    }

    #[test]
    fn test_find_owners_and_tags_for_file_empty_entries() {
        let entries = vec![];
        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_and_tags_for_file(file_path, &entries).unwrap();
        assert!(result.0.is_empty());
        assert!(result.1.is_empty());
    }

    #[test]
    fn test_find_owners_and_tags_for_file_simple_match() {
        let expected_owner = create_test_owner("@rust-team", OwnerType::Team);
        let expected_tag = create_test_tag("rust");
        let entries = vec![create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![expected_owner.clone()],
            vec![expected_tag.clone()],
        )];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_and_tags_for_file(file_path, &entries).unwrap();

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0], expected_owner);
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.1[0], expected_tag);
    }

    #[test]
    fn test_find_owners_and_tags_for_file_directory_hierarchy() {
        let root_owner = create_test_owner("@root-team", OwnerType::Team);
        let root_tag = create_test_tag("root");
        let src_owner = create_test_owner("@src-team", OwnerType::Team);
        let src_tag = create_test_tag("source");

        let entries = vec![
            create_test_codeowners_entry_matcher(
                "/project/CODEOWNERS",
                1,
                "*",
                vec![root_owner.clone()],
                vec![root_tag.clone()],
            ),
            create_test_codeowners_entry_matcher(
                "/project/src/CODEOWNERS",
                1,
                "*.rs",
                vec![src_owner.clone()],
                vec![src_tag.clone()],
            ),
        ];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_and_tags_for_file(file_path, &entries).unwrap();

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0], src_owner);
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.1[0], src_tag);
    }

    #[test]
    fn test_find_owners_and_tags_for_file_line_number_priority() {
        let general_owner = create_test_owner("@general-team", OwnerType::Team);
        let general_tag = create_test_tag("general");
        let specific_owner = create_test_owner("@specific-team", OwnerType::Team);
        let specific_tag = create_test_tag("specific");

        let entries = vec![
            create_test_codeowners_entry_matcher(
                "/project/CODEOWNERS",
                1,
                "*",
                vec![general_owner.clone()],
                vec![general_tag.clone()],
            ),
            create_test_codeowners_entry_matcher(
                "/project/CODEOWNERS",
                10,
                "src/*.rs",
                vec![specific_owner.clone()],
                vec![specific_tag.clone()],
            ),
        ];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_and_tags_for_file(file_path, &entries).unwrap();

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0], specific_owner);
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.1[0], specific_tag);
    }

    #[test]
    fn test_find_owners_and_tags_for_file_valid_pattern() {
        let entries = vec![create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            2,
            "*.rs",
            vec![create_test_owner("@team2", OwnerType::Team)],
            vec![create_test_tag("tag2")],
        )];

        let file_path = Path::new("/project/src/main.rs");
        let result = find_owners_and_tags_for_file(file_path, &entries).unwrap();

        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].identifier, "@team2");
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.1[0].0, "tag2");
    }
}
