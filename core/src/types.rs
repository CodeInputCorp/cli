use std::path::PathBuf;

/// CODEOWNERS entry with source tracking
#[derive(Debug)]
pub struct CodeownersEntry {
    pub source_file: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// Detailed owner representation
#[derive(Debug, Clone)]
pub struct Owner {
    pub identifier: String,
    pub owner_type: OwnerType,
}

/// Owner type classification
#[derive(Debug, Clone)]
pub enum OwnerType {
    User,
    Team,
    Email,
    Unowned,
    Unknown,
}

/// Tag representation
#[derive(Debug, Clone)]
pub struct Tag(pub String);

#[derive(Clone, Debug, PartialEq)]
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
