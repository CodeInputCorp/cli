use crate::common::find_files;
use crate::types::OutputFormat;

use utils::app_config::AppConfig;
use utils::error::Result;

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}

/// Preprocess CODEOWNERS files and build ownership map
pub fn codeowners_parse(
    path: &std::path::Path, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    println!("Parsing CODEOWNERS files at {}", path.display());

    let codeowners_files = crate::common::find_codeowners_files(path)?;

    dbg!(&codeowners_files);

    let parsed_codeowners = codeowners_files
        .iter()
        .filter_map(|file| {
            let parsed = crate::common::parse_codeowners(file).ok()?;
            Some((file, parsed))
        })
        .collect::<Vec<_>>();

    dbg!(&parsed_codeowners);

    let files = find_files(path)?;

    //dbg!(&files);

    println!("CODEOWNERS parsing completed successfully");
    Ok(())
}

/// Find and list files with their owners based on filter criteria
pub fn codeowners_list_files(
    path: Option<&std::path::Path>, tags: Option<&str>, owners: Option<&str>, unowned: bool,
    format: &OutputFormat,
) -> Result<()> {
    let path_str = path.map_or(".".into(), |p| p.display().to_string());
    info!("Listing files in {}", path_str);
    info!("Tags filter: {:?}", tags);
    info!("Owners filter: {:?}", owners);
    info!("Unowned only: {}", unowned);
    info!("Output format: {}", format);

    println!("Files listing completed");
    Ok(())
}

/// Display aggregated owner statistics and associations
pub fn codeowners_list_owners(format: &OutputFormat) -> Result<()> {
    info!("Output format: {}", format);

    println!("Owners listing completed");
    Ok(())
}

/// Audit and analyze tag usage across CODEOWNERS files
pub fn codeowners_list_tags(format: &OutputFormat) -> Result<()> {
    info!("Output format: {}", format);

    println!("Tags listing completed");
    Ok(())
}
