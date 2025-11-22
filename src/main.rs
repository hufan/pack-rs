mod app;
mod model;
mod packaging;
mod pipeline;
mod repo_ops;

use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.toml"));

    app::run(&config_path).await
}
