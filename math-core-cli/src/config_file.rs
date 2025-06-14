use std::{fs, io, path::Path};

use math_core::MathCoreConfig;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten)]
    pub math_core: MathCoreConfig,
}

/// Error type for configuration loading operations.
#[derive(Debug)]
pub enum ConfigError {
    /// I/O error when reading the file.
    Io(io::Error),
    /// TOML parsing error.
    Parse(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(err) => write!(f, "I/O error: {}", err),
            ConfigError::Parse(err) => write!(f, "TOML parsing error: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(err) => Some(err),
            ConfigError::Parse(err) => Some(err),
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::Parse(err)
    }
}

/// Loads and deserializes the MathCore configuration from a TOML file.
///
/// # Arguments
///
/// * `path` - The path to the TOML configuration file.
///
/// # Returns
///
/// Returns `Ok(MathCoreConfig)` on success, or `Err(ConfigError)` if the file
/// cannot be read or parsed.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
///
/// let config_path = Path::new("mathcore.toml");
/// match load_config_file(config_path) {
///     Ok(config) => println!("Loaded config: {:?}", config),
///     Err(e) => eprintln!("Failed to load config: {}", e),
/// }
/// ```
pub fn load_config_file(path: &Path) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(path)?;
    let config = parse_config(&content)?;
    Ok(config)
}

#[inline]
fn parse_config(s: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(s)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use math_core::PrettyPrint;

    use super::*;

    #[test]
    fn test_full_config() {
        let toml_content = r#"
pretty-print = "always"

[macros]
R = "\\mathbb{R}"
"é" = "\\acute{e}"
        "#;
        let config = parse_config(toml_content).unwrap();
        assert!(matches!(config.math_core.pretty_print, PrettyPrint::Always));
        assert_eq!(config.math_core.macros.get("R").unwrap(), "\\mathbb{R}");
        assert_eq!(config.math_core.macros.get("é").unwrap(), "\\acute{e}");
    }

    #[test]
    fn test_invalid_config() {
        let invalid_toml = "invalid_toml";
        let result = parse_config(invalid_toml);
        assert!(matches!(result, Err(ConfigError::Parse(_))));
    }

    #[test]
    fn test_partial_config() {
        let toml_content = r#"
[macros]
R = "\\mathbb{R}"
        "#;
        let config = parse_config(toml_content).unwrap();
        assert!(matches!(config.math_core.pretty_print, PrettyPrint::Never));
        assert_eq!(config.math_core.macros.get("R").unwrap(), "\\mathbb{R}");
    }
}
