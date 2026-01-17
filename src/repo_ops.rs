use crate::model::Repository;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;
use tokio::time::{sleep, timeout};

#[async_trait]
pub trait RepositoryFetcher: Send + Sync {
    async fn fetch(&self, repo: &Repository, max_retries: u32, timeout_secs: u64) -> Result<PathBuf>;
}

pub struct GitFetcher;

#[async_trait]
impl RepositoryFetcher for GitFetcher {
    async fn fetch(&self, repo: &Repository, max_retries: u32, timeout_secs: u64) -> Result<PathBuf> {
        let mut last_error = None;
        let repo_name = extract_repo_name(&repo.url)?;

        for attempt in 1..=max_retries {
            println!("[{}] Attempt {}/{}", repo_name, attempt, max_retries);

            let result = timeout(
                Duration::from_secs(timeout_secs),
                self.fetch_with_retry(repo)
            ).await;

            match result {
                Ok(Ok(path)) => {
                    if attempt > 1 {
                        println!("[{}] Successfully fetched on attempt {}", repo_name, attempt);
                    }
                    return Ok(path);
                }
                Ok(Err(e)) => {
                    let error = anyhow::anyhow!("Fetch failed: {}", e);
                    eprintln!("[{}] Attempt {}/{} failed: {}", repo_name, attempt, max_retries, error);
                    last_error = Some(error);
                }
                Err(_) => {
                    let error = anyhow::anyhow!("Fetch timeout after {} seconds", timeout_secs);
                    eprintln!("[{}] Attempt {}/{} timed out", repo_name, attempt, max_retries);
                    last_error = Some(error);
                }
            }

            if attempt < max_retries {
                let delay = Duration::from_secs(5 * attempt as u64);
                println!("[{}] Waiting {:?} before retry...", repo_name, delay);
                sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Fetch failed after {} attempts", max_retries)))
    }
}

impl GitFetcher {
    async fn fetch_with_retry(&self, repo: &Repository) -> Result<PathBuf> {
        let target_dir = get_repo_dir(repo)?;
        let target_path = target_dir.as_path();
        let target_path_str = target_path.to_str().ok_or_else(|| {
            anyhow!(
                "Target path contains invalid UTF-8: {}",
                target_path.display()
            )
        })?;

        if target_path.exists() && repo.clean_before_pull {
            println!("  Cleaning directory: {}", target_path.display());
            fs::remove_dir_all(target_path).await.with_context(|| {
                format!("Failed to remove directory: {}", target_path.display())
            })?;
        }

        if !target_path.exists() {
            println!(
                "  Cloning repository: {} -> {}",
                repo.url,
                target_path.display()
            );

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            let mut cmd = Command::new("git");
            cmd.args(&["clone", "-b", &repo.branch]);

            if let Some(depth) = repo.depth {
                cmd.args(&["--depth", &depth.to_string()]);
                println!("  Using shallow clone: depth={}", depth);
            }

            cmd.arg(&repo.url).arg(target_path_str);

            let status = cmd
                .status()
                .await
                .context("Failed to run git clone. Ensure git is installed.")?;

            if !status.success() {
                anyhow::bail!("git clone failed: {}", repo.url);
            }

            println!("  Clone completed");
        } else {
            println!("  Updating repository: {}", target_path.display());

            let checkout_status = Command::new("git")
                .args(&["-C", target_path_str, "checkout", &repo.branch])
                .status()
                .await
                .context("Failed to run git checkout")?;

            if !checkout_status.success() {
                println!(
                    "  Warning: failed to checkout branch {}, pulling current branch",
                    repo.branch
                );
            }

            let pull_status = Command::new("git")
                .args(&["-C", target_path_str, "pull"])
                .status()
                .await
                .context("Failed to run git pull")?;

            if !pull_status.success() {
                anyhow::bail!("git pull failed: {}", target_path.display());
            }

            println!("  Update completed");
        }

        Ok(target_dir)
    }
}

/// 从 Git URL 中提取仓库名称
pub fn extract_repo_name(url: &str) -> Result<String> {
    let url_trimmed = url.trim_end_matches(".git");

    if let Some(repo_name) = url_trimmed.split('/').last() {
        if !repo_name.is_empty() && !repo_name.contains(':') {
            return Ok(repo_name.to_string());
        }
    }

    if let Some(repo_name) = url_trimmed.split(':').last() {
        if !repo_name.is_empty() {
            return Ok(repo_name.to_string());
        }
    }

    anyhow::bail!("Failed to extract repository name from URL: {}", url)
}

fn get_repo_dir(repo: &Repository) -> Result<PathBuf> {
    if let Some(ref target_dir) = repo.target_dir {
        Ok(PathBuf::from(target_dir))
    } else {
        let repo_name = extract_repo_name(&repo.url)?;
        Ok(PathBuf::from("repos").join(repo_name))
    }
}

/// 获取用于打包的文件名（不包含扩展名）
/// 如果指定了 target_dir，则使用 target_dir 中的目录名
/// 否则使用从 URL 提取的仓库名
pub fn get_package_name(repo: &Repository) -> Result<String> {
    if let Some(ref target_dir) = repo.target_dir {
        let path = PathBuf::from(target_dir);
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                return Ok(name_str.to_string());
            }
        }
        // 如果无法从路径提取文件名，fallback 到 URL 提取
        extract_repo_name(&repo.url)
    } else {
        extract_repo_name(&repo.url)
    }
}
