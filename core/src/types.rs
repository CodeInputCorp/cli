use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// CODEOWNERS entry with source tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeownersEntry {
    pub source_file: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// Detailed owner representation
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Owner {
    pub identifier: String,
    pub owner_type: OwnerType,
}

/// Owner type classification
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum OwnerType {
    User,
    Team,
    Email,
    Unowned,
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

/// Tag representation
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Tag(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
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
/// File entry in the ownership cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// Cache for storing parsed CODEOWNERS information
#[derive(Debug)]
pub struct CodeownersCache {
    pub hash: [u8; 32],
    pub entries: Vec<CodeownersEntry>,
    pub files: Vec<FileEntry>,
    // Derived data for lookups
    pub owners_map: std::collections::HashMap<Owner, Vec<PathBuf>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEncoding {
    Bincode,
    Json,
}
