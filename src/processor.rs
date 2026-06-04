use image::imageops::FilterType::Lanczos3;
use std::path::{Path, PathBuf};

const MAX_WIDTH: u32 = 800;

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
pub fn process(
    input: &Path,
    output_dir: &Path,
    quality: f32,
) -> Result<ProcessResult, Box<dyn std::error::Error>> {
    let img = image::open(input)?;
    let (w, h) = (img.width(), img.height());

    let (new_w, new_h) = if w > MAX_WIDTH {
        let ratio = MAX_WIDTH as f64 / f64::from(w);
        (
            (f64::from(w) * ratio) as u32,
            (f64::from(h) * ratio) as u32,
        )
    } else {
        (w, h)
    };

    let resized = if (new_w, new_h) == (w, h) {
        img
    } else {
        img.resize_exact(new_w, new_h, Lanczos3)
    };

    let stem = input
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output_path = output_dir.join(format!("{stem}.webp"));

    let rgb = resized.to_rgba8();
    let encoder = webp::Encoder::from_rgba(&rgb, rgb.width(), rgb.height());
    let webp_mem = encoder.encode(quality);
    std::fs::write(&output_path, &*webp_mem)?;

    Ok(ProcessResult {
        input_path: input.to_path_buf(),
        output_path,
        original_width: w,
        original_height: h,
        final_width: new_w,
        final_height: new_h,
    })
}
