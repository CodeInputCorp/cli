//! Display utilities for CLI output formatting.
//!
//! This module contains functions for formatting and truncating text content
//! to fit within terminal display constraints while maintaining readability.

/// Truncates a file path to fit within the specified maximum length while preserving readability.
///
/// This function intelligently truncates paths by prioritizing the filename and including
/// as much of the path prefix as possible. When truncation is needed, it uses "..." to
/// indicate the omitted portion.
///
/// # Arguments
///
/// * `path` - The file path to truncate
/// * `max_len` - Maximum allowed length for the truncated path
///
/// # Returns
///
/// A truncated path string that fits within `max_len` characters
///
/// # Examples
///
/// ```
/// use crate::core::display::truncate_path;
///
/// // No truncation needed
/// assert_eq!(truncate_path("short.txt", 20), "short.txt");
///
/// // Preserves filename and path start
/// assert_eq!(truncate_path("./path/very-long-path/file.txt", 20), "./pa.../file.txt");
///
/// // Handles paths without slashes
/// assert_eq!(truncate_path("very-long-filename", 10), "very-lo...");
/// ```
pub(crate) fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        // Find the last slash to preserve filename
        if let Some(last_slash) = path.rfind('/') {
            let filename = &path[last_slash..]; // includes the slash

            // If filename itself is too long, truncate it
            if filename.len() >= max_len {
                let available_chars = max_len.saturating_sub(3);
                if available_chars == 0 {
                    "...".to_string()
                } else {
                    let chars: Vec<char> = filename.chars().collect();
                    let end_idx = (chars.len()).min(available_chars + 3); // +3 to account for the "..." we'll replace
                    let truncated: String = chars[3..end_idx].iter().collect(); // skip first 3 chars after "..."
                    format!("...{}", truncated)
                }
            } else {
                // Filename fits, now figure out how much path start we can include
                let remaining_space = max_len.saturating_sub(filename.len()).saturating_sub(3); // reserve 3 for "..."

                if remaining_space == 0 {
                    format!("...{}", filename)
                } else {
                    let path_chars: Vec<char> = path.chars().collect();
                    let path_start: String = path_chars[..remaining_space].iter().collect();
                    format!("{}...{}", path_start, filename)
                }
            }
        } else {
            // No slash found, just truncate normally
            let available_chars = max_len.saturating_sub(3);
            if available_chars == 0 {
                "...".to_string()
            } else {
                let chars: Vec<char> = path.chars().collect();
                let truncated: String = chars[..available_chars].iter().collect();
                format!("{}...", truncated)
            }
        }
    }
}

/// Truncates a string to fit within the specified maximum length.
///
/// This function performs simple string truncation by keeping the beginning
/// of the string and appending "..." to indicate truncation. It properly
/// handles Unicode characters by working with character boundaries.
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_len` - Maximum allowed length for the truncated string
///
/// # Returns
///
/// A truncated string that fits within `max_len` characters
///
/// # Examples
///
/// ```
/// use crate::core::display::truncate_string;
///
/// // No truncation needed
/// assert_eq!(truncate_string("short", 10), "short");
///
/// // Basic truncation
/// assert_eq!(truncate_string("this is a long string", 10), "this is...");
///
/// // Very short limit
/// assert_eq!(truncate_string("hello", 3), "...");
///
pub(crate) fn truncate_string(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();

    if chars.len() <= max_len {
        s.to_string()
    } else {
        let available_chars = max_len.saturating_sub(3);
        if available_chars == 0 {
            "...".to_string()
        } else {
            let truncated: String = chars[..available_chars].iter().collect();
            format!("{}...", truncated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_path_no_truncation_needed() {
        assert_eq!(truncate_path("short.txt", 20), "short.txt");
        assert_eq!(truncate_path("./file.rs", 10), "./file.rs");
        assert_eq!(truncate_path("a/b", 3), "a/b");
        assert_eq!(truncate_path("", 10), "");
        assert_eq!(truncate_path("a", 5), "a");
    }

    #[test]
    fn test_truncate_path_basic_truncation() {
        // Basic case: preserve filename and start of path
        assert_eq!(
            truncate_path("./path/very-long-path/file_name.pt", 25),
            "./path/ve.../file_name.pt"
        );
        assert_eq!(
            truncate_path("/usr/local/bin/program", 15),
            "/usr.../program"
        );
        assert_eq!(
            truncate_path("src/components/header.tsx", 20),
            "src/co.../header.tsx"
        );
    }

    #[test]
    fn test_truncate_path_filename_takes_most_space() {
        // When filename is long but fits, use remaining space for path start
        assert_eq!(
            truncate_path("./long-path/long-filename.txt", 25),
            "./lo.../long-filename.txt"
        );
        assert_eq!(
            truncate_path("/a/b/c/d/important.config", 20),
            ".../important.config"
        );
    }

    #[test]
    fn test_truncate_path_filename_barely_fits() {
        // When filename leaves very little space for path
        assert_eq!(
            truncate_path("./very-long-path/file.txt", 12),
            ".../file.txt"
        );
        assert_eq!(truncate_path("/extremely/long/path/f.x", 10), "/ex.../f.x");
    }

    #[test]
    fn test_truncate_path_no_slash() {
        // No slash in path - treat as regular string truncation
        assert_eq!(
            truncate_path("very-long-filename-without-slash", 15),
            "very-long-fi..."
        );
        assert_eq!(truncate_path("filename.txt", 8), "filen...");
        assert_eq!(truncate_path("toolong", 5), "to...");
    }

    #[test]
    fn test_truncate_path_edge_cases_with_slashes() {
        // Multiple slashes and various path structures
        assert_eq!(truncate_path("../../../../file.txt", 15), "../.../file.txt");
        assert_eq!(truncate_path("/", 5), "/");
        assert_eq!(truncate_path("./", 5), "./");
        assert_eq!(truncate_path("a/", 5), "a/");
        assert_eq!(truncate_path("/a", 5), "/a");
    }

    #[test]
    fn test_truncate_string_no_truncation_needed() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("exact", 5), "exact");
        assert_eq!(truncate_string("", 10), "");
        assert_eq!(truncate_string("a", 5), "a");
        assert_eq!(truncate_string("ab", 2), "ab");
    }

    #[test]
    fn test_truncate_string_basic_truncation() {
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("testing", 6), "tes...");
    }

    #[test]
    fn test_truncate_string_minimal_length() {
        // Very short max_len cases
        assert_eq!(truncate_string("hello", 3), "...");
        assert_eq!(truncate_string("toolong", 4), "t...");
        assert_eq!(truncate_string("ab", 3), "ab");
    }

    #[test]
    fn test_truncate_string_unicode() {
        // Test with unicode characters (current implementation may have issues)
        assert_eq!(truncate_string("café", 4), "café");
        assert_eq!(truncate_string("hello 世界", 8), "hello 世界");
    }
}
