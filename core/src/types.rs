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
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeownersCache {
    pub entries: Vec<CodeownersEntry>,
    pub files: Vec<FileEntry>,
    // Derived data for lookups
    pub owners_map: std::collections::HashMap<Owner, Vec<PathBuf>>,
    pub tags_map: std::collections::HashMap<Tag, Vec<PathBuf>>,
}
