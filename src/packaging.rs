use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

#[async_trait]
pub trait Packager: Send + Sync {
    async fn package(
        &self,
        source_dir: &Path,
        output_path: &Path,
        top_level_dir: &str,
        include_git: bool,
    ) -> Result<()>;
}

pub struct TarPackager;

#[async_trait]
impl Packager for TarPackager {
    async fn package(
        &self,
        source_dir: &Path,
        output_path: &Path,
        top_level_dir: &str,
        include_git: bool,
    ) -> Result<()> {
        let source_dir = source_dir
            .canonicalize()
            .with_context(|| format!("Failed to resolve directory: {}", source_dir.display()))?;
        let output_path = match output_path.canonicalize() {
            Ok(path) => path,
            Err(_) => output_path.to_path_buf(),
        };

        let parent_dir = source_dir
            .parent()
            .context("Source directory has no parent; cannot run tar")?
            .to_path_buf();
        let source_name = source_dir
            .file_name()
            .context("Failed to get source directory name")?
            .to_string_lossy()
            .to_string();

        let parent_dir_str = parent_dir.to_string_lossy().to_string();
        let output_path_str = output_path.to_string_lossy().to_string();

        let mut cmd = Command::new("tar");
        cmd.arg("-czf")
            .arg(&output_path_str)
            .arg("-C")
            .arg(&parent_dir_str);

        if source_name != top_level_dir {
            cmd.arg("--transform")
                .arg(format!("s@^{}@{}@", source_name, top_level_dir));
        }

        if !include_git {
            cmd.arg("--exclude=.git").arg("--exclude=*/.git");
        }

        cmd.arg(&source_name);

        let status = cmd
            .status()
            .await
            .context("Failed to run tar command. Ensure tar is installed.")?;

        if !status.success() {
            anyhow::bail!("tar command failed; output file: {}", output_path.display());
        }

        Ok(())
    }
}
