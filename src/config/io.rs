//! Functions for loading config from a file.

use std::path;

/// Gets a path to the default config file.
#[must_use]
pub fn default_file() -> path::PathBuf {
    let mut path = default_dir();
    path.push("config.toml");
    path
}

/// Gets a path to the default config directory.
#[must_use]
pub fn default_dir() -> path::PathBuf {
    // TODO(@MattWindsor91): make this an error
    dirs::config_dir().map_or_else(path::PathBuf::new, |mut ucd| {
        ucd.push("phph");
        ucd
    })
}
