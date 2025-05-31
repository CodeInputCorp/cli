use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// Module-level comments
//! # Core Data Types
//!
//! This module defines the primary data structures and enumerations used throughout the
//! core logic of the application. These types are essential for representing `CODEOWNERS`
//! file rules, ownership information, cached data, and various configuration options
//! like output formats and cache encodings.
//!
//! The main types include:
//! - `CodeownersEntry`: Represents a single rule from a `CODEOWNERS` file.
//! - `Owner`: Details an owner, including their identifier and type.
//! - `OwnerType`: Categorizes owners (e.g., User, Team, Email).
//! - `Tag`: Represents a tag associated with a `CODEOWNERS` rule.
//! - `FileEntry`: Stores ownership and tag information for a specific file path in the cache.
//! - `CodeownersCache`: The main cache structure holding parsed entries, file ownership data,
//!   and aggregated lookup maps for owners and tags.
//! - `OutputFormat`: Enum for specifying how command output should be formatted.
//! - `CacheEncoding`: Enum for specifying the serialization format for the cache.

/// Represents a single parsed entry (rule) from a `CODEOWNERS` file.
///
/// Each entry links a file pattern to a set of owners and tags, and also tracks
/// its origin (source file and line number) for better traceability and debugging.
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeownersEntry {
    /// The path to the `CODEOWNERS` file from which this entry was parsed.
    pub source_file: PathBuf,
    /// The line number in the `source_file` where this entry was defined.
    pub line_number: usize,
    /// The file path pattern (e.g., `*.rs`, `/docs/`) that this rule applies to.
    pub pattern: String,
    /// A list of `Owner`s associated with this pattern.
    pub owners: Vec<Owner>,
    /// A list of `Tag`s associated with this pattern.
    pub tags: Vec<Tag>,
}

/// Represents an owner, identified by a string and classified by an `OwnerType`.
///
/// Owners can be individuals (users, emails) or groups (teams).
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Owner {
    /// The unique identifier for the owner.
    /// Examples: `@username`, `@org/team-name`, `user@example.com`.
    pub identifier: String,
    /// The type of the owner (e.g., User, Team, Email).
    pub owner_type: OwnerType,
}

/// Enumerates the different types of owners that can be specified in a `CODEOWNERS` file.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum OwnerType {
    /// A GitHub user, typically prefixed with `@` (e.g., `@username`).
    User,
    /// A GitHub team, typically in the format `@organization/team-name`.
    Team,
    /// An email address.
    Email,
    /// Indicates that a pattern has no designated owner (e.g., using `NOOWNER`).
    Unowned,
    /// The owner type could not be determined from the identifier string.
    Unknown,
}

impl std::fmt::Display for OwnerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerType::User => write!(f, "User"),
            OwnerType::Team => write!(f, "Team"),
            OwnerType::Email => write!(f, "Email"),
            OwnerType::Unowned => write!(f, "Unowned"),
            OwnerType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Represents a tag associated with a `CODEOWNERS` rule.
///
/// Tags are simple string identifiers, often used for categorization or
/// additional metadata (e.g., `#frontend`, `#security`). The leading `#`
/// is part of the tag's representation in the file but is typically stripped
/// for storage in this struct.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Tag(
    /// The name of the tag (e.g., "frontend", "security").
    pub String,
);

/// Specifies the desired format for command output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    /// Human-readable plain text, often formatted as a table.
    Text,
    /// Machine-readable JSON format.
    Json,
    /// Machine-readable Bincode format (binary serialization).
    Bincode,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Bincode => write!(f, "bincode"),
        }
    }
}

// Cache related types

/// Represents a file within the repository and its associated owners and tags,
/// as determined by the `CODEOWNERS` rules. This struct is primarily used within
/// the `CodeownersCache`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// The path to the file within the repository.
    pub path: PathBuf,
    /// A list of `Owner`s determined for this file.
    pub owners: Vec<Owner>,
    /// A list of `Tag`s determined for this file.
    pub tags: Vec<Tag>,
}

/// The main cache structure for storing processed `CODEOWNERS` data.
///
/// This cache holds all parsed entries, a list of all files with their resolved
/// ownership, and pre-computed maps for quick lookups of files by owner or tag.
/// It also stores a repository hash to help determine if the cache is stale.
///
/// Note: This struct has custom `Serialize` and `Deserialize` implementations
/// to handle the `HashMap` fields correctly, as `serde`'s default derive
/// might not be optimal or directly applicable for all map key types without
/// specific feature flags or wrapper types.
#[derive(Debug)]
pub struct CodeownersCache {
    /// A hash representing the state of the repository when the cache was built.
    /// Used to validate cache freshness.
    pub hash: [u8; 32],
    /// A list of all original `CodeownersEntry` items parsed from all `CODEOWNERS` files.
    pub entries: Vec<CodeownersEntry>,
    /// A list of `FileEntry` items, where each entry details a specific file's path,
    /// its resolved owners, and its resolved tags.
    pub files: Vec<FileEntry>,
    /// A map where keys are `Owner`s and values are lists of `PathBuf`s
    /// representing the files owned by that owner. This is derived data for quick lookups.
    pub owners_map: std::collections::HashMap<Owner, Vec<PathBuf>>,
    /// A map where keys are `Tag`s and values are lists of `PathBuf`s
    /// representing the files associated with that tag. This is derived data for quick lookups.
    pub tags_map: std::collections::HashMap<Tag, Vec<PathBuf>>,
}

impl Serialize for CodeownersCache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("CodeownersCache", 4)?;
        state.serialize_field("hash", &self.hash)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("files", &self.files)?;

        // Convert owners_map to a serializable format
        let owners_map_serializable: Vec<(&Owner, &Vec<PathBuf>)> =
            self.owners_map.iter().collect();
        state.serialize_field("owners_map", &owners_map_serializable)?;

        // Convert tags_map to a serializable format
        let tags_map_serializable: Vec<(&Tag, &Vec<PathBuf>)> = self.tags_map.iter().collect();
        state.serialize_field("tags_map", &tags_map_serializable)?;

        state.end()
    }
}

impl<'de> Deserialize<'de> for CodeownersCache {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CodeownersCacheHelper {
            hash: [u8; 32],
            entries: Vec<CodeownersEntry>,
            files: Vec<FileEntry>,
            owners_map: Vec<(Owner, Vec<PathBuf>)>,
            tags_map: Vec<(Tag, Vec<PathBuf>)>,
        }

        let helper = CodeownersCacheHelper::deserialize(deserializer)?;

        // Convert back to HashMap
        let mut owners_map = std::collections::HashMap::new();
        for (owner, paths) in helper.owners_map {
            owners_map.insert(owner, paths);
        }

        let mut tags_map = std::collections::HashMap::new();
        for (tag, paths) in helper.tags_map {
            tags_map.insert(tag, paths);
        }

        Ok(CodeownersCache {
            hash: helper.hash,
            entries: helper.entries,
            files: helper.files,
            owners_map,
            tags_map,
        })
    }
}

/// Specifies the encoding format for serializing and deserializing the `CodeownersCache`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEncoding {
    /// Bincode: A compact binary serialization format.
    Bincode,
    /// Json: A human-readable JSON format.
    Json,
}
