use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub output_dir: String,
    #[allow(dead_code)]
    // package_prefix: String,
    pub repositories: Vec<Repository>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Repository {
    pub name: String,
    pub url: String,
    pub branch: String,
    #[serde(default)]
    pub target_dir: Option<String>,
    #[serde(default)]
    pub clean_before_pull: bool,
    #[serde(default)]
    pub depth: Option<u32>,
    #[serde(default)]
    pub include_git: bool,
}
