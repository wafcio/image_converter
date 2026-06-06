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
    clippy::cast_lossless
)]
pub fn process(
    input: &Path,
    output_dir: &Path,
    format: OutputFormat,
    quality: f32,
    width: Option<u32>,
    height: Option<u32>,
    output_name: Option<&str>,
) -> Result<ProcessResult, Box<dyn std::error::Error>> {
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
            let webp_mem = encoder.encode(quality);
            std::fs::write(&output_path, &*webp_mem)?;
        }
        OutputFormat::Avif => {
            let file = std::fs::File::create(&output_path)?;
            let encoder =
                image::codecs::avif::AvifEncoder::new_with_speed_quality(file, 4, quality as u8);
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
