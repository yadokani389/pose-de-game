use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage};

pub(crate) struct PreprocessedInput {
    pub(crate) data: Vec<f32>,
    pub(crate) letterbox: LetterboxInfo,
    pub(crate) input_size: u32,
}

pub(crate) struct LetterboxInfo {
    pub(crate) scale: f32,
    pub(crate) pad_x: u32,
    pub(crate) pad_y: u32,
    pub(crate) orig_w: u32,
    pub(crate) orig_h: u32,
    pub(crate) new_w: u32,
    pub(crate) new_h: u32,
    pub(crate) input_size: u32,
}

pub(crate) fn preprocess(
    frame_w: u32,
    frame_h: u32,
    rgba: Vec<u8>,
    input_size: u32,
) -> Result<PreprocessedInput> {
    let rgba_image =
        image::RgbaImage::from_raw(frame_w, frame_h, rgba).context("failed to build RGBA image")?;
    let rgb_image = DynamicImage::ImageRgba8(rgba_image).to_rgb8();

    let scale = input_size as f32 / frame_w.max(frame_h) as f32;
    let new_w = (frame_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (frame_h as f32 * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(
        &rgb_image,
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let mut padded = RgbImage::new(input_size, input_size);
    let pad_x = (input_size - new_w) / 2;
    let pad_y = (input_size - new_h) / 2;
    image::imageops::replace(&mut padded, &resized, pad_x as i64, pad_y as i64);

    let mut data = vec![0.0f32; (input_size * input_size * 3) as usize];
    let stride = (input_size * input_size) as usize;
    for y in 0..input_size {
        for x in 0..input_size {
            let pixel = padded.get_pixel(x, y);
            let idx = (y * input_size + x) as usize;
            data[idx] = pixel[0] as f32 / 255.0;
            data[stride + idx] = pixel[1] as f32 / 255.0;
            data[stride * 2 + idx] = pixel[2] as f32 / 255.0;
        }
    }

    Ok(PreprocessedInput {
        data,
        letterbox: LetterboxInfo {
            scale,
            pad_x,
            pad_y,
            orig_w: frame_w,
            orig_h: frame_h,
            new_w,
            new_h,
            input_size,
        },
        input_size,
    })
}

impl LetterboxInfo {
    pub(crate) fn to_original(&self, x: f32, y: f32) -> (f32, f32) {
        let mut ox = (x - self.pad_x as f32) / self.scale;
        let mut oy = (y - self.pad_y as f32) / self.scale;
        ox = ox.clamp(0.0, self.orig_w.saturating_sub(1) as f32);
        oy = oy.clamp(0.0, self.orig_h.saturating_sub(1) as f32);
        (ox, oy)
    }
}
