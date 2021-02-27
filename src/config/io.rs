//! Functions for loading config from a file.

use std::path;

/// Gets a path to the default config file.
pub fn default_file() -> path::PathBuf {
    let mut path = default_dir();
    path.push("config.toml");
    path
}

/// Gets a path to the default config directory.
pub fn default_dir() -> path::PathBuf {
    // TODO(@MattWindsor91): make this an error
    if let Some(mut ucd) = dirs::config_dir() {
        ucd.push("phph");
        ucd
    } else {
        path::PathBuf::new()
    }
}
