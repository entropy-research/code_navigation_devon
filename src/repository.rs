use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Repository {
    pub disk_path: PathBuf,
}

impl Repository {
    pub fn from_path(path: &Path) -> Result<Self> {
        let disk_path = path.canonicalize().context("failed to canonicalize path")?;
        Ok(Self { disk_path })
    }
}