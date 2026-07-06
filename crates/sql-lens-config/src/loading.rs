use crate::{ConfigLoadError, SqlLensConfig};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl SqlLensConfig {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigLoadError> {
        let path = path.as_ref();
        let input = fs::read_to_string(path).map_err(|source| ConfigLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        Self::from_toml_str_with_path(&input, Some(path.to_path_buf()))
    }

    pub fn from_toml_str(input: &str) -> Result<Self, ConfigLoadError> {
        Self::from_toml_str_with_path(input, None)
    }

    fn from_toml_str_with_path(
        input: &str,
        path: Option<PathBuf>,
    ) -> Result<Self, ConfigLoadError> {
        toml::from_str(input).map_err(|source| ConfigLoadError::Parse { path, source })
    }
}
