use std::path::Path;

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub heuristics: HeuristicsConfig,

    /// Automatic quality search (tries multiple quality levels per image)
    #[serde(default)]
    pub quality_search: QualitySearchConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct HeuristicsConfig {
    /// Enable automatic per-file parameter selection
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Thresholds for small images (e.g. icons)
    #[serde(default)]
    pub small: CategoryConfig,

    /// Thresholds for large images (e.g. product photos)
    #[serde(default)]
    pub large: CategoryConfig,

    /// Fallback for images that are neither small nor large
    #[serde(default)]
    pub medium: CategoryConfig,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct CategoryConfig {
    /// Max width (inclusive) for this category (unbounded if absent)
    #[serde(default)]
    pub max_width: Option<u32>,

    /// Max height (inclusive) for this category (unbounded if absent)
    #[serde(default)]
    pub max_height: Option<u32>,

    /// Minimum dimension (exclusive) — image qualifies if either side exceeds this
    #[serde(default)]
    pub min_dimension: Option<u32>,

    /// Minimum file size in bytes (exclusive)
    #[serde(default)]
    pub min_file_size: Option<u64>,

    /// Use lossless encoding
    #[serde(default)]
    pub lossless: bool,

    /// Encoding quality (0–100)
    #[serde(default = "default_quality")]
    pub quality: f32,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct QualitySearchConfig {
    /// Enable automatic quality search (tries multiple quality levels per image)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }
fn default_quality() -> f32 { 80.0 }

impl Default for HeuristicsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            small: CategoryConfig {
                max_width: Some(256),
                max_height: Some(256),
                min_dimension: None,
                min_file_size: None,
                lossless: true,
                quality: 100.0,
            },
            large: CategoryConfig {
                max_width: None,
                max_height: None,
                min_dimension: Some(1200),
                min_file_size: Some(1_048_576),
                lossless: false,
                quality: 75.0,
            },
            medium: CategoryConfig {
                max_width: None,
                max_height: None,
                min_dimension: None,
                min_file_size: None,
                lossless: false,
                quality: 80.0,
            },
        }
    }
}

impl Config {
    /// Load config from `config.toml` in CWD. Returns `None` if the file doesn't exist.
    pub fn load() -> Option<Self> {
        let path = Path::new("config.toml");
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        let cfg: Config = toml::from_str(&content).ok()?;
        Some(cfg)
    }
}
