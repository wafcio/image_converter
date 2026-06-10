use crate::config::{HeuristicsConfig, QualitySearchConfig};
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
    #[must_use]
    pub fn extension(self) -> &'static str {
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
///
/// When no heuristics are provided, returns the default quality (80.0, lossy).
///
/// ```
/// use image_converter::image;
/// use image_converter::processor;
///
/// let dir = std::env::temp_dir().join("image_converter_doc_detect");
/// std::fs::create_dir_all(&dir).unwrap();
/// let path = dir.join("icon.png");
/// image::RgbaImage::new(64, 64).save(&path).unwrap();
///
/// let result = processor::detect_best_config(&path, None).unwrap();
/// assert!(!result.lossless);
/// assert_eq!(result.quality, 80.0);
///
/// std::fs::remove_dir_all(&dir).ok();
/// ```
///
/// # Errors
///
/// Returns an error if the image file cannot be read or its dimensions cannot be determined.
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

/// Encode an image to memory and return the encoded bytes.
///
/// When `lossless` is true:
/// - WebP: uses `encode_lossless_with_quality` where `quality` controls
///   compression effort (higher = more effort, smaller file, slower).
/// - AVIF: encodes at quality=100 (near-lossless / best available).
///
/// When `lossless` is false (lossy): `quality` controls visual quality
/// (higher = better quality, larger file).
///
/// # Errors
///
/// Returns an error if encoding fails.
pub fn encode_to_memory(
    img: &image::DynamicImage,
    format: OutputFormat,
    quality: u8,
    speed: u8,
    lossless: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    match format {
        OutputFormat::Webp => {
            let rgb = img.to_rgba8();
            let encoder = webp::Encoder::from_rgba(&rgb, rgb.width(), rgb.height());
            Ok(encoder.encode_simple(lossless, f32::from(quality)).unwrap().to_vec())
        }
        OutputFormat::Avif => {
            let q = if lossless { 100u8 } else { quality };
            let mut buf = std::io::Cursor::new(Vec::new());
            let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
                &mut buf,
                speed,
                q,
            );
            encoder.write_image(
                img.as_bytes(),
                img.width(),
                img.height(),
                img.color().into(),
            )?;
            Ok(buf.into_inner())
        }
    }
}

/// Search for the best quality level by encoding in-memory at multiple trial levels
/// and picking the one with the best size-to-quality ratio (diminishing returns).
///
/// Tries qualities 70, 80, 90. Picks the lowest quality where stepping up yields
/// less than 15% size reduction.
///
/// ```
/// use image_converter::image;
/// use image_converter::processor;
///
/// let img = image::DynamicImage::new_rgba8(100, 100);
/// let quality = processor::search_quality(&img, processor::OutputFormat::Webp, 6);
/// assert!((70.0..=90.0).contains(&quality));
/// ```
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn search_quality(img: &image::DynamicImage, format: OutputFormat, speed: u8) -> f32 {
    let qualities = [70u8, 80, 90];
    let sizes: Vec<usize> = qualities
        .iter()
        .filter_map(|&q| encode_to_memory(img, format, q, speed, false).ok())
        .map(|v| v.len())
        .collect();

    for i in 0..qualities.len().saturating_sub(1) {
        let improvement = 1.0 - sizes[i + 1] as f64 / sizes[i] as f64;
        if improvement < 0.15 {
            return f32::from(qualities[i]);
        }
    }

    f32::from(qualities[qualities.len() - 1])
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::too_many_arguments
)]
///
/// # Errors
///
/// Returns an error if the input image cannot be read, the output directory cannot be written to,
/// or the encoding process fails.
pub fn process(
    input: &Path,
    output_dir: &Path,
    format: OutputFormat,
    quality: f32,
    width: Option<u32>,
    height: Option<u32>,
    output_name: Option<&str>,
    heuristics: Option<&HeuristicsConfig>,
    quality_search: Option<&QualitySearchConfig>,
    speed: u8,
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

    let final_quality = if !lossless && quality_search.is_some_and(|qs| qs.enabled) {
        let searched = search_quality(&resized, format, speed);
        if heuristics.is_some() {
            eprintln!(
                "  search: {} quality={:.0}",
                input.file_name().unwrap_or_default().to_string_lossy(),
                searched,
            );
        }
        searched
    } else {
        effective_quality
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

    let data = encode_to_memory(&resized, format, final_quality as u8, speed, lossless)?;
    std::fs::write(&output_path, &data)?;

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

        let cfg = HeuristicsConfig { enabled: false, ..Default::default() };
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

    #[test]
    fn test_search_quality_webp() {
        let img = image::DynamicImage::new_rgba8(400, 300);
        let q = search_quality(&img, OutputFormat::Webp, 6);
        assert!((70.0..=90.0).contains(&q), "quality should be 70–90, got {q}");
    }

    #[test]
    fn test_search_quality_avif() {
        let img = image::DynamicImage::new_rgba8(400, 300);
        let q = search_quality(&img, OutputFormat::Avif, 6);
        assert!((70.0..=90.0).contains(&q), "quality should be 70–90, got {q}");
    }

    #[test]
    fn test_search_quality_returns_lower_for_small_image() {
        // Smaller images tend to get lower quality (diminishing returns kicks in earlier)
        let small = image::DynamicImage::new_rgba8(100, 100);
        let large = image::DynamicImage::new_rgba8(2000, 1500);
        let q_small = search_quality(&small, OutputFormat::Webp, 6);
        let q_large = search_quality(&large, OutputFormat::Webp, 6);
        assert!(
            q_small <= q_large,
            "small image should get <= quality of large image ({q_small} vs {q_large})",
        );
    }
}
