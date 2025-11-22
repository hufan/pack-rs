use crate::model::Repository;
use crate::packaging::Packager;
use crate::repo_ops::{extract_repo_name, get_package_name, RepositoryFetcher};
use anyhow::{bail, Result};
use futures::future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinError;

pub struct Pipeline<F, P>
where
    F: RepositoryFetcher + 'static,
    P: Packager + 'static,
{
    fetcher: Arc<F>,
    packager: Arc<P>,
    output_dir: PathBuf,
}

impl<F, P> Pipeline<F, P>
where
    F: RepositoryFetcher + 'static,
    P: Packager + 'static,
{
    pub fn new(fetcher: F, packager: P, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            fetcher: Arc::new(fetcher),
            packager: Arc::new(packager),
            output_dir: output_dir.into(),
        }
    }

    pub async fn run(&self, repositories: &[Repository]) -> Result<()> {
        let tasks: Vec<_> = repositories
            .iter()
            .map(|repo| {
                let repo = repo.clone();
                let fetcher = Arc::clone(&self.fetcher);
                let packager = Arc::clone(&self.packager);
                let output_dir = self.output_dir.clone();

                tokio::spawn(async move {
                    let repo_name = extract_repo_name(&repo.url)?;
                    let package_base_name = get_package_name(&repo)?;

                    println!("\n[{}] Processing repository: {}", repo_name, repo.name);

                    let repo_dir = fetcher.fetch(&repo).await?;

                    if !repo_dir.exists() {
                        eprintln!("[{}] Warning: repo directory missing, skipping", repo_name);
                        return Ok(());
                    }

                    let package_name = format!("{}.tar.gz", package_base_name);
                    let package_path = Path::new(&output_dir).join(&package_name);

                    packager
                        .package(&repo_dir, &package_path, &package_base_name, repo.include_git)
                        .await
                })
            })
            .collect();

        let results: Vec<Result<Result<(), anyhow::Error>, JoinError>> =
            future::join_all(tasks).await;

        let mut has_error = false;
        for result in results {
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    eprintln!("Task failed: {}", e);
                    has_error = true;
                }
                Err(e) => {
                    eprintln!("Task panicked: {}", e);
                    has_error = true;
                }
            }
        }

        if has_error {
            bail!("Some repositories failed. See errors above.");
        }

        Ok(())
    }
}
