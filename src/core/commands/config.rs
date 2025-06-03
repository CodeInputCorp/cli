use crate::utils::{app_config::AppConfig, error::Result};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct ConfigDisplay {
    #[tabled(rename = "Setting")]
    key: String,
    #[tabled(rename = "Value")]
    value: String,
}

/// Show the configuration file
pub(crate) fn run() -> Result<()> {
    let config = AppConfig::fetch()?;

    let table_data = vec![
        ConfigDisplay {
            key: "Debug Mode".to_string(),
            value: config.debug.to_string(),
        },
        ConfigDisplay {
            key: "Log Level".to_string(),
            value: config.log_level.to_string(),
        },
        ConfigDisplay {
            key: "Cache File".to_string(),
            value: config.cache_file,
        },
    ];

    let mut table = Table::new(table_data);
    table.with(tabled::settings::Style::modern());

    println!("{}", table);

    Ok(())
}
