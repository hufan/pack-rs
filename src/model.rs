use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub output_dir: String,
    #[allow(dead_code)]
    // package_prefix: String,
    pub repositories: Vec<Repository>,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout: u64,
    #[serde(default = "default_package_timeout")]
    pub package_timeout: u64,
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
    #[serde(default = "default_enable_fetch")]
    pub enable_fetch: bool,
    #[serde(default = "default_enable_package")]
    pub enable_package: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout: u64,
    #[serde(default = "default_package_timeout")]
    pub package_timeout: u64,
}

fn default_enable_fetch() -> bool {
    true
}

fn default_enable_package() -> bool {
    true
}

fn default_max_concurrent() -> usize {
    4
}

fn default_max_retries() -> u32 {
    3
}

fn default_fetch_timeout() -> u64 {
    1800
}

fn default_package_timeout() -> u64 {
    600
}
