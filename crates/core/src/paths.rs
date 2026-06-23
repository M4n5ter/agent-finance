use std::path::PathBuf;

use anyhow::{Result, anyhow};
use directories::ProjectDirs;

pub fn config_dir() -> Result<PathBuf> {
    project_dirs()
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or_else(|| anyhow!("failed to resolve agent-finance config directory"))
}

pub fn data_dir() -> Result<PathBuf> {
    project_dirs()
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or_else(|| anyhow!("failed to resolve agent-finance data directory"))
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("io.github", "m4n5ter", "agent-finance")
}
