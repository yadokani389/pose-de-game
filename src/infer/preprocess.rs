use anyhow::{Context, Result};
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};

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
    let scale = input_size as f32 / frame_w.max(frame_h) as f32;
    let new_w = (frame_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (frame_h as f32 * scale).round().max(1.0) as u32;
    let pad_x = (input_size - new_w) / 2;
    let pad_y = (input_size - new_h) / 2;

    let src_image = ImageRef::new(frame_w, frame_h, &rgba, PixelType::U8x4)
        .context("failed to build resize input image")?;
    let mut resized = Image::new(new_w, new_h, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
        .use_alpha(false);
    resizer
        .resize(&src_image, &mut resized, Some(&options))
        .context("failed to resize input image")?;

    let mut data = vec![0.0f32; (input_size * input_size * 3) as usize];
    let stride = (input_size * input_size) as usize;
    let resized_buf = resized.buffer();
    let dst_row_stride = input_size as usize;
    let src_row_stride = new_w as usize;
    for y in 0..new_h {
        let dst_row = (y + pad_y) as usize * dst_row_stride;
        let src_row = y as usize * src_row_stride;
        for x in 0..new_w {
            let dst_idx = dst_row + (x + pad_x) as usize;
            let src_idx = (src_row + x as usize) * 4;
            data[dst_idx] = resized_buf[src_idx] as f32 / 255.0;
            data[stride + dst_idx] = resized_buf[src_idx + 1] as f32 / 255.0;
            data[stride * 2 + dst_idx] = resized_buf[src_idx + 2] as f32 / 255.0;
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
