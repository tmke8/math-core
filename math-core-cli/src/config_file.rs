use std::{fs, io, path::Path};

use math_core::MathCoreConfig;

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
pub fn load_config_file(path: &Path) -> Result<MathCoreConfig, ConfigError> {
    let content = fs::read_to_string(path)?;
    let config: MathCoreConfig = toml::from_str(&content)?;
    Ok(config)
}
