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

/// Automatic quality search configuration.
///
/// Encodes at multiple quality levels (from `max_quality` down to `min_quality`),
/// skipping any level where the encoded output is **larger than the original file**.
/// For acceptable levels, computes SSIM between original and decoded image, and
/// picks the lowest quality where SSIM >= `ssim_threshold`. If no level passes both
/// checks, the original file is copied as-is.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct QualitySearchConfig {
    /// Enable automatic quality search.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// SSIM threshold (0.0–1.0). A level is accepted when SSIM >= this value.
    /// Higher = more faithful to original. Default: 0.97.
    #[serde(default = "default_ssim_threshold")]
    pub ssim_threshold: f64,

    /// Minimum quality to try (0–100).
    #[serde(default = "default_min_quality")]
    pub min_quality: u8,

    /// Maximum quality to try (0–100).
    #[serde(default = "default_max_quality")]
    pub max_quality: u8,

    /// Step between quality levels when scanning.
    #[serde(default = "default_step")]
    pub step: u8,
}

fn default_ssim_threshold() -> f64 { 0.97 }
fn default_min_quality() -> u8 { 70 }
fn default_max_quality() -> u8 { 95 }
fn default_step() -> u8 { 5 }

impl Default for QualitySearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ssim_threshold: default_ssim_threshold(),
            min_quality: default_min_quality(),
            max_quality: default_max_quality(),
            step: default_step(),
        }
    }
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
    #[must_use]
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
