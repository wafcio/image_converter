use crate::config::HeuristicsConfig;
use clap::ValueEnum;
use image::imageops::FilterType::Lanczos3;
use image::ImageEncoder;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Webp,
    Avif,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

impl OutputFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Webp => "webp",
            Self::Avif => "avif",
        }
    }
}

pub struct ProcessResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub original_width: u32,
    pub original_height: u32,
    pub final_width: u32,
    pub final_height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct HeuristicResult {
    pub quality: f32,
    pub lossless: bool,
}

/// Determine the best encoding parameters based on image dimensions and config.
///
/// Uses `image::image_dimensions()` (header-only) to avoid decoding the full image
/// just for size classification.
pub fn detect_best_config(
    input: &Path,
    heuristics: Option<&HeuristicsConfig>,
) -> Result<HeuristicResult, Box<dyn std::error::Error + Send + Sync>> {
    let Some(h) = heuristics else {
        return Ok(HeuristicResult {
            quality: 80.0,
            lossless: false,
        });
    };

    if !h.enabled {
        return Ok(HeuristicResult {
            quality: h.medium.quality,
            lossless: h.medium.lossless,
        });
    }

    let (w, h_px) = image::image_dimensions(input)?;
    let file_size = std::fs::metadata(input)?.len();

    // Small category: both dimensions at or below threshold
    if let Some(true) = h.small.max_width.map_or(Some(false), |mw| Some(w <= mw))
        && let Some(true) = h.small.max_height.map_or(Some(false), |mh| Some(h_px <= mh))
    {
        return Ok(HeuristicResult {
            quality: h.small.quality,
            lossless: h.small.lossless,
        });
    }

    // Large category: either dimension exceeds threshold or file is big
    let is_large_dim = h
        .large
        .min_dimension
        .is_some_and(|md| w > md || h_px > md);
    let is_large_file = h
        .large
        .min_file_size
        .is_some_and(|ms| file_size > ms);
    if is_large_dim || is_large_file {
        return Ok(HeuristicResult {
            quality: h.large.quality,
            lossless: h.large.lossless,
        });
    }

    // Fallback: medium
    Ok(HeuristicResult {
        quality: h.medium.quality,
        lossless: h.medium.lossless,
    })
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
fn resolve_dimensions(
    orig_w: u32,
    orig_h: u32,
    width: Option<u32>,
    height: Option<u32>,
) -> (u32, u32) {
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let h = (f64::from(orig_h) * f64::from(w) / f64::from(orig_w)).round() as u32;
            (w, h.max(1))
        }
        (None, Some(h)) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let w = (f64::from(orig_w) * f64::from(h) / f64::from(orig_h)).round() as u32;
            (w.max(1), h)
        }
        (None, None) => (orig_w, orig_h),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::too_many_arguments
)]
pub fn process(
    input: &Path,
    output_dir: &Path,
    format: OutputFormat,
    quality: f32,
    width: Option<u32>,
    height: Option<u32>,
    output_name: Option<&str>,
    heuristics: Option<&HeuristicsConfig>,
) -> Result<ProcessResult, Box<dyn std::error::Error + Send + Sync>> {
    let heur = detect_best_config(input, heuristics)?;
    let effective_quality = if heuristics.is_some() { heur.quality } else { quality };
    let lossless = heur.lossless;

    let img = image::open(input)?;
    let (w, h) = (img.width(), img.height());

    let (new_w, new_h) = resolve_dimensions(w, h, width, height);

    let resized = if (new_w, new_h) == (w, h) {
        img
    } else {
        img.resize_exact(new_w, new_h, Lanczos3)
    };

    let stem = output_name.map_or_else(
        || {
            input
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        },
        String::from,
    );
    let output_path = output_dir.join(format!("{}.{}", stem, format.extension()));

    match format {
        OutputFormat::Webp => {
            let rgb = resized.to_rgba8();
            let encoder = webp::Encoder::from_rgba(&rgb, rgb.width(), rgb.height());
            if lossless {
                let webp_mem = encoder.encode_lossless();
                std::fs::write(&output_path, &*webp_mem)?;
            } else {
                let webp_mem = encoder.encode(effective_quality);
                std::fs::write(&output_path, &*webp_mem)?;
            }
        }
        OutputFormat::Avif => {
            let file = std::fs::File::create(&output_path)?;
            let q = if lossless {
                100u8
            } else {
                effective_quality as u8
            };
            let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(file, 4, q);
            encoder.write_image(
                resized.as_bytes(),
                resized.width(),
                resized.height(),
                resized.color().into(),
            )?;
        }
    }

    Ok(ProcessResult {
        input_path: input.to_path_buf(),
        output_path,
        original_width: w,
        original_height: h,
        final_width: new_w,
        final_height: new_h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_no_heuristics() {
        let dir = std::env::temp_dir().join("test_no_heuristic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        let img = image::RgbaImage::new(200, 200);
        img.save(&path).unwrap();

        let result = detect_best_config(&path, None).unwrap();
        assert!(!result.lossless);
        assert_eq!(result.quality, 80.0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_small_image() {
        let dir = std::env::temp_dir().join("test_small_heuristic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("icon.png");
        let img = image::RgbaImage::new(64, 64);
        img.save(&path).unwrap();

        let cfg = HeuristicsConfig::default();
        let result = detect_best_config(&path, Some(&cfg)).unwrap();
        assert!(result.lossless);
        assert_eq!(result.quality, 100.0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_large_image() {
        let dir = std::env::temp_dir().join("test_large_heuristic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("photo.png");
        let img = image::RgbaImage::new(1920, 1080);
        img.save(&path).unwrap();

        let cfg = HeuristicsConfig::default();
        let result = detect_best_config(&path, Some(&cfg)).unwrap();
        assert!(!result.lossless);
        assert_eq!(result.quality, 75.0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_medium_image() {
        let dir = std::env::temp_dir().join("test_medium_heuristic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("medium.png");
        let img = image::RgbaImage::new(500, 500);
        img.save(&path).unwrap();

        let cfg = HeuristicsConfig::default();
        let result = detect_best_config(&path, Some(&cfg)).unwrap();
        assert!(!result.lossless);
        assert_eq!(result.quality, 80.0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_heuristic_disabled() {
        let dir = std::env::temp_dir().join("test_disabled_heuristic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("icon.png");
        let img = image::RgbaImage::new(64, 64);
        img.save(&path).unwrap();

        let mut cfg = HeuristicsConfig::default();
        cfg.enabled = false;
        let result = detect_best_config(&path, Some(&cfg)).unwrap();
        assert!(!result.lossless);
        assert_eq!(result.quality, 80.0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_resolve_dimensions_both_some() {
        assert_eq!(resolve_dimensions(100, 200, Some(50), Some(60)), (50, 60));
    }

    #[test]
    fn test_resolve_dimensions_width_only() {
        let (w, h) = resolve_dimensions(800, 600, Some(400), None);
        assert_eq!(w, 400);
        assert_eq!(h, 300);
    }

    #[test]
    fn test_resolve_dimensions_height_only() {
        let (w, h) = resolve_dimensions(800, 600, None, Some(300));
        assert_eq!(w, 400);
        assert_eq!(h, 300);
    }

    #[test]
    fn test_resolve_dimensions_none() {
        assert_eq!(resolve_dimensions(100, 200, None, None), (100, 200));
    }
}
