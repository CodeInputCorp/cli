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
