use crate::model::Config;
use crate::packaging::TarPackager;
use crate::pipeline::Pipeline;
use crate::repo_ops::GitFetcher;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

pub async fn run(config_path: &Path) -> Result<()> {
    println!("Starting packaging workflow...");

    let config_content = fs::read_to_string(config_path)
        .await
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = toml::from_str(&config_content).context("Failed to parse config file")?;

    println!(
        "Loaded config. Found {} repositories",
        config.repositories.len()
    );
    println!("Max concurrent operations: {}", config.max_concurrent);
    println!("Max retries: {}", config.max_retries);
    println!("Fetch timeout: {}s", config.fetch_timeout);
    println!("Package timeout: {}s", config.package_timeout);

    let output_path = Path::new(&config.output_dir);
    if !output_path.exists() {
        fs::create_dir_all(output_path)
            .await
            .with_context(|| format!("Failed to create output dir: {}", output_path.display()))?;
    }

    println!(
        "\nProcessing {} repositories with max {} concurrent...",
        config.repositories.len(),
        config.max_concurrent
    );

    let pipeline = Pipeline::new(GitFetcher, TarPackager, &config.output_dir, config.max_concurrent);
    pipeline.run(&config.repositories).await?;

    println!("\nAll repositories processed.");

    Ok(())
}
