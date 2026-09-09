#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::rng::Pcg32;

const SUBSAMPLES: u32 = 4;

pub struct Raster {
    width: u32,
    height: u32,
    pixels: Vec<f32>,
}

impl Raster {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0.0; (width * height * 4) as usize],
        }
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32)
    }

    pub fn bounds(&self) -> Rect {
        Rect::from_corners(Vec2::ZERO, self.size())
    }

    /// `inside` takes pixel coordinates and returns a signed distance in pixels;
    /// `shade` takes coordinates normalized to `-1..1` and returns straight RGBA.
    pub fn shape(&mut self, inside: impl Fn(Vec2) -> f32, shade: impl Fn(Vec2) -> [f32; 4]) {
        let step = 1.0 / SUBSAMPLES as f32;
        let half = self.size() * 0.5;
        for y in 0..self.height {
            for x in 0..self.width {
                let mut coverage = 0.0;
                for sy in 0..SUBSAMPLES {
                    for sx in 0..SUBSAMPLES {
                        let point = Vec2::new(
                            x as f32 + (sx as f32 + 0.5) * step,
                            y as f32 + (sy as f32 + 0.5) * step,
                        );
                        if inside(point) < 0.0 {
                            coverage += 1.0;
                        }
                    }
                }
                if coverage == 0.0 {
                    continue;
                }
                coverage /= (SUBSAMPLES * SUBSAMPLES) as f32;

                let center = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let source = shade((center - half) / half);
                self.blend(x, y, source, coverage);
            }
        }
    }

    fn blend(&mut self, x: u32, y: u32, source: [f32; 4], coverage: f32) {
        let alpha = source[3] * coverage;
        if alpha <= 0.0 {
            return;
        }
        let index = ((y * self.width + x) * 4) as usize;
        let target = &mut self.pixels[index..index + 4];
        let out_alpha = alpha + target[3] * (1.0 - alpha);
        for channel in 0..3 {
            target[channel] =
                (source[channel] * alpha + target[channel] * target[3] * (1.0 - alpha)) / out_alpha;
        }
        target[3] = out_alpha;
    }

    /// Wide, low-contrast alpha ramps band in the 8-bit framebuffer; a little
    /// per-texel noise hides the steps.
    pub fn jitter_alpha(&mut self, amount: f32, rng: &mut Pcg32) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            if pixel[3] > 0.0 {
                pixel[3] = (pixel[3] + rng.range(-amount, amount)).clamp(0.0, 1.0);
            }
        }
    }

    pub fn finish(self, images: &mut Assets<Image>) -> Handle<Image> {
        let data = self
            .pixels
            .iter()
            .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let mut image = Image::new(
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        images.add(image)
    }
}

pub fn ellipse(center: Vec2, radius: Vec2) -> impl Fn(Vec2) -> f32 {
    move |point| {
        let normalized = (point - center) / radius;
        (normalized.length() - 1.0) * radius.min_element()
    }
}

pub fn rect(bounds: Rect) -> impl Fn(Vec2) -> f32 {
    move |point| {
        let distance = (bounds.min - point).max(point - bounds.max);
        distance.max(Vec2::ZERO).length() + distance.max_element().min(0.0)
    }
}

/// Four-pointed sparkle: a superellipse with a concave exponent.
pub fn sparkle(center: Vec2, radius: f32, sharpness: f32) -> impl Fn(Vec2) -> f32 {
    move |point| {
        let normalized = (point - center).abs() / radius;
        (normalized.x.powf(sharpness) + normalized.y.powf(sharpness) - 1.0) * radius
    }
}

pub fn solid(color: [f32; 4]) -> impl Fn(Vec2) -> [f32; 4] {
    move |_| color
}

pub fn radial(inner: f32, outer: f32, alpha: f32, falloff: f32) -> impl Fn(Vec2) -> [f32; 4] {
    move |uv| {
        let distance = uv.length();
        let ramp = ((distance - inner) / (outer - inner)).clamp(0.0, 1.0);
        [1.0, 1.0, 1.0, alpha * (1.0 - ramp).powf(falloff)]
    }
}

pub fn radial_inverse(
    inner: f32,
    outer: f32,
    alpha: f32,
    falloff: f32,
) -> impl Fn(Vec2) -> [f32; 4] {
    move |uv| {
        let distance = uv.length();
        let ramp = ((distance - inner) / (outer - inner)).clamp(0.0, 1.0);
        [1.0, 1.0, 1.0, alpha * ramp.powf(falloff)]
    }
}
