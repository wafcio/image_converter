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
            let mem = encoder
                .encode_simple(lossless, f32::from(quality))
                .map_err(|e| format!("WebP encoding failed: {e:?}"))?;
            Ok(mem.to_vec())
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

/// Compute SSIM (Structural Similarity) between original and decoded image.
/// Returns 0.0–1.0 where 1.0 = identical.
fn compute_ssim(original: &image::DynamicImage, decoded: &image::DynamicImage) -> f64 {
    let orig_rgb = original.to_rgb8();
    let dec_rgb = decoded.to_rgb8();

    let (w, h) = orig_rgb.dimensions();
    if w == 0 || h == 0 {
        return 0.0;
    }

    let Ok(img1) = iqa::Image::srgb8(w, h, orig_rgb.to_vec()) else { return 0.0 };
    let Ok(img2) = iqa::Image::srgb8(w, h, dec_rgb.to_vec()) else { return 0.0 };

    iqa::ssim(&img1, &img2, iqa::SsimOptions::default()).unwrap_or(1.0)
}

/// Search for the best quality level by encoding in-memory at multiple trial levels.
///
/// Iterates from `max_quality` down to `min_quality` (configured via `QualitySearchConfig`).
/// For each level:
///   1. Encodes the image to memory.
///   2. If encoded size > original file size, skips (would make file larger).
///   3. Decodes the encoded bytes and computes SSIM against the original.
///   4. If SSIM >= `ssim_threshold`, accepts this quality level (lowest passing = best).
///   5. Levels below the first accepted one are skipped — we want the lowest quality
///      that still looks good.
///
/// If **no** level passes both checks (too large + SSIM below threshold), returns `None`
/// — the caller should copy the original file as-is.
///
/// ```
/// use image_converter::image;
/// use image_converter::config::QualitySearchConfig;
/// use image_converter::processor;
///
/// let img = image::DynamicImage::new_rgba8(100, 100);
/// let cfg = QualitySearchConfig::default();
/// let quality = processor::search_quality(&img, processor::OutputFormat::Webp, 6, &cfg, None);
/// // With no original_size guard, should find an acceptable level.
/// assert!(quality.is_some());
/// ```
#[must_use]
pub fn search_quality(
    img: &image::DynamicImage,
    format: OutputFormat,
    speed: u8,
    config: &QualitySearchConfig,
    original_size: Option<u64>,
) -> Option<f32> {
    let mut q = config.max_quality;
    while q >= config.min_quality {
        let Ok(encoded) = encode_to_memory(img, format, q, speed, false) else {
            q = q.saturating_sub(config.step);
            continue;
        };

        // Size guard: encoded must not exceed original
        if let Some(max_size) = original_size
            && (encoded.len() as u64) > max_size
        {
            q = q.saturating_sub(config.step);
            continue;
        }

        // SSIM check: skip if we can't decode
        let Ok(decoded) = image::load_from_memory(&encoded) else {
            q = q.saturating_sub(config.step);
            continue;
        };

        if compute_ssim(img, &decoded) >= config.ssim_threshold {
            // Lowest acceptable quality — this is optimal
            return Some(f32::from(q));
        }

        // SSIM too low at this quality — try higher quality
        q = q.saturating_sub(config.step);
    }

    // No level passed both size and SSIM checks
    None
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
/// When `quality_search` is enabled, encodes the image at multiple quality levels and
/// picks the lowest level where the output size does not exceed the original file and
/// SSIM >= threshold. If no level passes both checks, the original file is copied as-is.
///
/// Returns an error if the input image cannot be read, the output directory cannot be written to,
/// or the encoding process fails.
///
/// # Panics
///
/// Panics if `quality_search` is `Some` and `enabled: true` (the condition is checked
/// via `is_some_and` first, so this only happens on internal logic inconsistency).
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
    let original_size = std::fs::metadata(input).ok().map(|m| m.len());
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
        let qs = quality_search.unwrap(); // safe — checked above
        let searched = search_quality(&resized, format, speed, qs, original_size);
        if let Some(q) = searched {
            if heuristics.is_some() {
                eprintln!(
                    "  search: {} quality={:.0}",
                    input.file_name().unwrap_or_default().to_string_lossy(),
                    q,
                );
            }
            q
        } else {
            // No quality level passed both size + SSIM checks → copy original
            eprintln!(
                "  skip: {} — quality search found no acceptable level, copying original",
                input.file_name().unwrap_or_default().to_string_lossy(),
            );
            return Ok(ProcessResult {
                input_path: input.to_path_buf(),
                output_path: output_dir.join(
                    output_name.map_or_else(
                        || input.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        String::from,
                    )
                ),
                original_width: w,
                original_height: h,
                final_width: new_w,
                final_height: new_h,
            });
        }
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
        let cfg = QualitySearchConfig::default();
        let q = search_quality(&img, OutputFormat::Webp, 6, &cfg, None);
        assert!(q.is_some(), "quality search should find a level");
        let q = q.unwrap();
        assert!((70.0..=95.0).contains(&q), "quality should be 70–95, got {q}");
    }

    #[test]
    fn test_search_quality_avif() {
        let img = image::DynamicImage::new_rgba8(400, 300);
        let cfg = QualitySearchConfig::default();
        let q = search_quality(&img, OutputFormat::Avif, 6, &cfg, None);
        assert!(q.is_some(), "quality search should find a level");
        let q = q.unwrap();
        assert!((70.0..=95.0).contains(&q), "quality should be 70–95, got {q}");
    }

    #[test]
    fn test_search_quality_returns_lower_for_small_image() {
        let small = image::DynamicImage::new_rgba8(100, 100);
        let large = image::DynamicImage::new_rgba8(2000, 1500);
        let cfg = QualitySearchConfig::default();
        let q_small = search_quality(&small, OutputFormat::Webp, 6, &cfg, None).unwrap();
        let q_large = search_quality(&large, OutputFormat::Webp, 6, &cfg, None).unwrap();
        assert!(
            q_small <= q_large,
            "small image should get <= quality of large image ({q_small} vs {q_large})",
        );
    }

    #[test]
    fn test_search_quality_size_guard() {
        // Small image: encode at low quality produces tiny output, high quality may be larger.
        // With original_size=1 we force every level to exceed it → expect None.
        let img = image::DynamicImage::new_rgba8(100, 100);
        let cfg = QualitySearchConfig::default();
        let q = search_quality(&img, OutputFormat::Webp, 6, &cfg, Some(1));
        assert!(q.is_none(), "size guard should reject all levels when original is tiny");
    }
}
