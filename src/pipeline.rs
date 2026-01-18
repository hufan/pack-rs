use crate::model::Repository;
use crate::packaging::Packager;
use crate::repo_ops::{extract_repo_name, get_package_name, get_repo_dir, RepositoryFetcher};
use anyhow::{bail, Result};
use futures::future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinError;

pub struct Pipeline<F, P>
where
    F: RepositoryFetcher + 'static,
    P: Packager + 'static,
{
    fetcher: Arc<F>,
    packager: Arc<P>,
    output_dir: PathBuf,
    max_concurrent: usize,
}

impl<F, P> Pipeline<F, P>
where
    F: RepositoryFetcher + 'static,
    P: Packager + 'static,
{
    pub fn new(fetcher: F, packager: P, output_dir: impl Into<PathBuf>, max_concurrent: usize) -> Self {
        Self {
            fetcher: Arc::new(fetcher),
            packager: Arc::new(packager),
            output_dir: output_dir.into(),
            max_concurrent,
        }
    }

    pub async fn run(&self, repositories: &[Repository]) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        
        let tasks: Vec<_> = repositories
            .iter()
            .map(|repo| {
                let repo = repo.clone();
                let fetcher = Arc::clone(&self.fetcher);
                let packager = Arc::clone(&self.packager);
                let output_dir = self.output_dir.clone();
                let semaphore = Arc::clone(&semaphore);

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        anyhow::anyhow!("Failed to acquire semaphore: {}", e)
                    })?;

                    let repo_name = extract_repo_name(&repo.url)?;
                    let package_base_name = get_package_name(&repo)?;

                    println!("\n[{}] Processing repository: {}", repo_name, repo.name);
                    println!("[{}] Stages: fetch={}, package={}", 
                             repo_name, repo.enable_fetch, repo.enable_package);

                    let max_retries = repo.max_retries;
                    let fetch_timeout = repo.fetch_timeout;
                    let package_timeout = repo.package_timeout;

                    let repo_dir = if repo.enable_fetch {
                        println!("[{}] Fetching...", repo_name);
                        Some(fetcher.fetch(&repo, max_retries, fetch_timeout).await?)
                    } else {
                        println!("[{}] Using existing directory", repo_name);
                        Some(get_repo_dir(&repo)?)
                    };

                    let repo_dir = match repo_dir {
                        Some(dir) => dir,
                        None => {
                            eprintln!("[{}] Warning: No repository directory, skipping", repo_name);
                            return Ok(());
                        }
                    };

                    if !repo_dir.exists() {
                        eprintln!("[{}] Warning: repo directory missing, skipping", repo_name);
                        return Ok(());
                    }

                    if repo.enable_package {
                        println!("[{}] Packaging...", repo_name);
                        let package_name = format!("{}.tar.gz", package_base_name);
                        let package_path = Path::new(&output_dir).join(&package_name);

                        packager
                            .package(&repo_dir, &package_path, &package_base_name, repo.include_git, package_timeout)
                            .await?;
                    } else {
                        println!("[{}] Package skipped", repo_name);
                    }

                    println!("[{}] Completed", repo_name);

                    Ok(())
                })
            })
            .collect();

        let results: Vec<Result<Result<(), anyhow::Error>, JoinError>> =
            future::join_all(tasks).await;

        let mut has_error = false;
        let mut success_count = 0;
        let mut failed_count = 0;

        for result in results {
            match result {
                Ok(Ok(_)) => {
                    success_count += 1;
                }
                Ok(Err(e)) => {
                    eprintln!("Task failed: {}", e);
                    has_error = true;
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("Task panicked: {}", e);
                    has_error = true;
                    failed_count += 1;
                }
            }
        }

        println!("\n==================== SUMMARY ====================");
        println!("Total repositories: {}", repositories.len());
        println!("Successfully processed: {}", success_count);
        println!("Failed: {}", failed_count);
        if failed_count > 0 {
            println!("Success rate: {:.1}%", (success_count as f64 / repositories.len() as f64) * 100.0);
        }
        println!("================================================");

        if has_error {
            bail!("Some repositories failed. See errors above.");
        }

        Ok(())
    }
}
