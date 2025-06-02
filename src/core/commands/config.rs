use crate::utils::{app_config::AppConfig, error::Result};

/// Show the configuration file
pub(crate) fn run() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}
