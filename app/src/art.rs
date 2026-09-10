use bevy::prelude::*;

use crate::geometry::{LINES, outer_h};
use crate::raster::{
    Raster, ellipse, in_pixels, outline, radial, radial_inverse, rect, rounded_rect, scaled,
    smooth_union, solid, sparkle, union,
};
use crate::rng::Pcg32;
use crate::theme::{
    BASE_H, BOTTLE_W, CORK_CAP_H, CORK_H, CORK_W, DROPLET_H, DROPLET_W, GLASS_WALL, ITEM_H, NECK_H,
};

const SCALE: u32 = 2;
const NOISE_SEED: u64 = 0xA17_5EED;
const GRADIENT_NOISE: f32 = 0.02;

pub const GLASS_PAD: f32 = 5.0;
pub const ITEM_W: f32 = BOTTLE_W - 2.0 * GLASS_WALL;
pub const ITEM_SURFACE_H: f32 = 7.0;

const BODY_TOP: f32 = 14.0;
const BODY_RADIUS: f32 = 12.0;
const NECK_HALF_W: f32 = 10.0;
const NECK_TOP: f32 = 2.0;
const RIM_HALF_W: f32 = 13.5;
const RIM_H: f32 = 9.0;
const SHOULDER_BLEND: f32 = 10.0;
const GLASS_STROKE: f32 = 3.0;
const ITEM_RADIUS: f32 = 7.0;
const STREAK_RADIUS: f32 = 3.5;
const STREAK_FEATHER: f32 = 3.0;
const STREAK_ALPHA: f32 = 0.5;

#[derive(Resource)]
pub struct Art {
    pub glow: Handle<Image>,
    pub vignette: Handle<Image>,
    pub star: Handle<Image>,
    pub item_body: Handle<Image>,
    pub item_base: Handle<Image>,
    pub item_surface: Handle<Image>,
    pub glass_back: [Handle<Image>; LINES],
    pub glass_front: [Handle<Image>; LINES],
    pub cork: Handle<Image>,
    pub cork_cap: Handle<Image>,
    pub droplet: Handle<Image>,
}

pub struct ArtPlugin;

impl Plugin for ArtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, build_art);
    }
}

pub fn glass_size(lines: u8) -> Vec2 {
    Vec2::new(BOTTLE_W, outer_h(lines)) + Vec2::splat(2.0 * GLASS_PAD)
}

fn build_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut rng = Pcg32::new(NOISE_SEED, 1);
    commands.insert_resource(Art {
        glow: glow(&mut images, &mut rng),
        vignette: vignette(&mut images, &mut rng),
        star: star(&mut images),
        item_body: item_body(&mut images),
        item_base: item_base(&mut images),
        item_surface: item_surface(&mut images),
        glass_back: std::array::from_fn(|line| glass_back(&mut images, line as u8 + 1)),
        glass_front: std::array::from_fn(|line| glass_front(&mut images, line as u8 + 1)),
        cork: cork(&mut images),
        cork_cap: cork_cap(&mut images),
        droplet: droplet(&mut images),
    });
}

fn raster(size: Vec2) -> Raster {
    Raster::new(
        (size.x * SCALE as f32) as u32,
        (size.y * SCALE as f32) as u32,
    )
}

fn glow(images: &mut Assets<Image>, rng: &mut Pcg32) -> Handle<Image> {
    let mut raster = Raster::new(256 * SCALE, 256 * SCALE);
    let half = raster.size() * 0.5;
    raster.shape(ellipse(half, half), radial(0.0, 1.0, 1.0, 2.4));
    raster.jitter_alpha(GRADIENT_NOISE, rng);
    raster.finish(images)
}

fn vignette(images: &mut Assets<Image>, rng: &mut Pcg32) -> Handle<Image> {
    let mut raster = Raster::new(256 * SCALE, 256 * SCALE);
    let bounds = raster.bounds();
    raster.shape(rect(bounds), radial_inverse(0.55, 1.35, 1.0, 1.6));
    raster.jitter_alpha(GRADIENT_NOISE, rng);
    raster.finish(images)
}

fn star(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = Raster::new(12 * SCALE, 12 * SCALE);
    let half = raster.size() * 0.5;
    raster.shape(ellipse(half, half * 0.5), radial(0.0, 0.5, 0.55, 2.0));
    raster.shape(
        sparkle(half, half.x - 0.5, 0.55),
        radial(0.3, 1.15, 1.0, 0.8),
    );
    raster.finish(images)
}

/// Cylinder shading baked into the luminance: the tint at draw time turns it
/// into a rounded body of liquid.
fn fluid_shade(uv: Vec2) -> f32 {
    let depth = 1.0 - 0.24 * uv.x * uv.x;
    let sheen = 0.14 * (-((uv.x + 0.45) / 0.26).powi(2)).exp();
    (depth + sheen).clamp(0.0, 1.0)
}

fn item_shade(uv: Vec2) -> [f32; 4] {
    let level = fluid_shade(uv) * 1.0f32.lerp(0.88, uv.y * 0.5 + 0.5);
    [level, level, level, 1.0]
}

fn item_body(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ITEM_W, ITEM_H));
    let bounds = raster.bounds();
    raster.shape(rect(bounds), item_shade);
    raster.finish(images)
}

fn item_base(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ITEM_W, ITEM_H));
    let bounds = Rect::new(0.0, 0.0, ITEM_W, ITEM_H);
    let straight = Rect::new(0.0, 0.0, ITEM_W, ITEM_H - ITEM_RADIUS);
    raster.shape(
        scaled(
            union(rect(straight), rounded_rect(bounds, ITEM_RADIUS)),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

/// Only the half of the surface ellipse that rises above the item, so its lower
/// edge is a straight seam that continues the slab shading exactly.
fn item_surface(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ITEM_W, ITEM_SURFACE_H));
    let size = raster.size();
    raster.shape(
        ellipse(
            Vec2::new(size.x * 0.5, size.y),
            Vec2::new(size.x * 0.5, size.y),
        ),
        |uv| {
            let level = (fluid_shade(uv) + 0.10 * (1.0 - (uv.y * 0.5 + 0.5))).clamp(0.0, 1.0);
            [level, level, level, 1.0]
        },
    );
    raster.finish(images)
}

fn bottle_sdf(lines: u8) -> impl Fn(Vec2) -> f32 {
    let height = outer_h(lines);
    let center = GLASS_PAD + BOTTLE_W * 0.5;
    let body = rounded_rect(
        Rect::new(
            GLASS_PAD,
            GLASS_PAD + BODY_TOP,
            GLASS_PAD + BOTTLE_W,
            GLASS_PAD + height,
        ),
        BODY_RADIUS,
    );
    let neck = rounded_rect(
        Rect::new(
            center - NECK_HALF_W,
            GLASS_PAD + NECK_TOP,
            center + NECK_HALF_W,
            GLASS_PAD + NECK_H + 4.0,
        ),
        5.0,
    );
    let rim = rounded_rect(
        Rect::new(
            center - RIM_HALF_W,
            GLASS_PAD,
            center + RIM_HALF_W,
            GLASS_PAD + RIM_H,
        ),
        4.0,
    );
    smooth_union(smooth_union(body, neck, SHOULDER_BLEND), rim, 2.0)
}

fn glass_back(images: &mut Assets<Image>, lines: u8) -> Handle<Image> {
    let size = glass_size(lines);
    let mut raster = raster(size);
    let pixels = raster.size();
    raster.shape(
        scaled(bottle_sdf(lines), SCALE as f32),
        in_pixels(pixels, {
            let sdf = scaled(bottle_sdf(lines), SCALE as f32);
            move |point| {
                let inside = (-sdf(point) / (GLASS_WALL * 2.0 * SCALE as f32)).clamp(0.0, 1.0);
                [1.0, 1.0, 1.0, 1.0f32.lerp(0.5, inside)]
            }
        }),
    );
    raster.finish(images)
}

fn glass_front(images: &mut Assets<Image>, lines: u8) -> Handle<Image> {
    let height = outer_h(lines);
    let size = glass_size(lines);
    let center = GLASS_PAD + BOTTLE_W * 0.5;
    let mut raster = raster(size);

    raster.shape(
        scaled(outline(bottle_sdf(lines), GLASS_STROKE), SCALE as f32),
        solid([1.0, 1.0, 1.0, 1.0]),
    );
    raster.shape(
        scaled(
            rounded_rect(
                Rect::new(
                    center - RIM_HALF_W,
                    GLASS_PAD,
                    center + RIM_HALF_W,
                    GLASS_PAD + RIM_H,
                ),
                4.0,
            ),
            SCALE as f32,
        ),
        solid([0.9, 0.9, 0.9, 0.7]),
    );
    let streak_top = GLASS_PAD + NECK_H + 8.0;
    let streak_bottom = streak_top + (height - NECK_H - BASE_H) * 0.45;
    let streak = Rect::new(GLASS_PAD + 9.0, streak_top, GLASS_PAD + 16.0, streak_bottom);
    let feather = rounded_rect(streak, STREAK_RADIUS);
    raster.shape(
        scaled(rounded_rect(streak, STREAK_RADIUS), SCALE as f32),
        in_pixels(size * SCALE as f32, move |point| {
            let point = point / SCALE as f32;
            let edge = (-feather(point) / STREAK_FEATHER).clamp(0.0, 1.0);
            let fade = ((point.y - streak_top) / (streak_bottom - streak_top)).clamp(0.0, 1.0);
            let head = (fade / 0.15).min(1.0);
            [1.0, 1.0, 1.0, STREAK_ALPHA * edge * head * (1.0 - fade)]
        }),
    );
    raster.finish(images)
}

const CORK_RADIUS: f32 = 5.0;
const CORK_INSET: f32 = 3.0;

fn cork(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(CORK_W, CORK_H));
    raster.shape(
        scaled(
            rounded_rect(
                Rect::new(CORK_INSET, CORK_CAP_H * 0.5, CORK_W - CORK_INSET, CORK_H),
                CORK_RADIUS,
            ),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

fn cork_cap(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(CORK_W, CORK_CAP_H));
    raster.shape(
        scaled(
            rounded_rect(Rect::new(0.0, 0.0, CORK_W, CORK_CAP_H), CORK_RADIUS),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

fn droplet(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(DROPLET_W, DROPLET_H));
    let size = raster.size();
    raster.shape(ellipse(size * 0.5, size * 0.5), item_shade);
    raster.finish(images)
}
