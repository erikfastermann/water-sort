use std::f32::consts::PI;

use bevy::prelude::*;

use crate::geometry::{LINES, outer_h};
use crate::raster::{
    Raster, ellipse, half_plane, in_pixels, intersect, outline, radial, radial_inverse, rect, ring,
    rounded_rect, scaled, smooth_union, solid, sparkle, triangle, union,
};
use crate::rng::Pcg32;
use crate::theme::{
    BAND_RADIUS, BAND_RIM_H, BAND_STROKE, BASE_H, BOTTLE_W, CONFETTI_H, CONFETTI_W, CORK_CAP_H,
    CORK_H, CORK_W, DROPLET_H, DROPLET_W, GLASS_WALL, HEADER_PILL, HEADER_PILL_BORDER,
    HEADER_PILL_RADIUS, ITEM_H, LID_H, LID_W, NAV_BUTTON, NAV_BUTTON_RADIUS, NAV_GLYPH_SIZE,
    NAV_PANEL, NAV_PANEL_BORDER, NAV_PANEL_RADIUS, NECK_H, NEXT_PILL, NEXT_PILL_BORDER,
    NEXT_PILL_RADIUS, PLUG_H, PLUG_W, QUESTION_H, QUESTION_W, ROCK_H, ROCK_W, ROPE_H, ROPE_W,
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
    pub question: Handle<Image>,
    pub metal_band: Handle<Image>,
    pub metal_lid: Handle<Image>,
    pub rock_base: Handle<Image>,
    pub rope: Handle<Image>,
    pub plug: Handle<Image>,
    pub droplet: Handle<Image>,
    pub header_pill: Plate,
    pub nav_panel: Plate,
    pub next_pill: Plate,
    pub nav_button: Handle<Image>,
    pub nav_undo: Handle<Image>,
    pub nav_next: Handle<Image>,
    pub confetti: Handle<Image>,
}

/// A bordered panel drawn as two stacked sprites, because one tinted grayscale
/// texture cannot carry two colours.
pub struct Plate {
    pub outer: Handle<Image>,
    pub inner: Handle<Image>,
    pub size: Vec2,
    pub border: f32,
}

impl Plate {
    pub fn inner_size(&self) -> Vec2 {
        self.size - Vec2::splat(2.0 * self.border)
    }
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
        question: question(&mut images),
        metal_band: metal_band(&mut images),
        metal_lid: metal_lid(&mut images),
        rock_base: rock_base(&mut images),
        rope: rope(&mut images),
        plug: plug(&mut images),
        droplet: droplet(&mut images),
        header_pill: plate(
            &mut images,
            HEADER_PILL,
            HEADER_PILL_RADIUS,
            HEADER_PILL_BORDER,
        ),
        nav_panel: plate(&mut images, NAV_PANEL, NAV_PANEL_RADIUS, NAV_PANEL_BORDER),
        next_pill: plate(&mut images, NEXT_PILL, NEXT_PILL_RADIUS, NEXT_PILL_BORDER),
        nav_button: rounded(&mut images, Vec2::splat(NAV_BUTTON), NAV_BUTTON_RADIUS),
        nav_undo: nav_undo(&mut images),
        nav_next: nav_next(&mut images),
        confetti: rounded(
            &mut images,
            Vec2::new(CONFETTI_W, CONFETTI_H),
            CONFETTI_W * 0.35,
        ),
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

const QUESTION_RADIUS: f32 = 6.0;
const QUESTION_STROKE: f32 = 5.0;

/// Arc open at the lower left, a stem continuing straight down from where the
/// arc terminates, and a dot.
fn question(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(QUESTION_W, QUESTION_H));
    let center = Vec2::new(QUESTION_W * 0.5, 10.0);
    let arc = union(
        intersect(
            ring(center, QUESTION_RADIUS, QUESTION_STROKE),
            half_plane(center, Vec2::Y),
        ),
        intersect(
            ring(center, QUESTION_RADIUS, QUESTION_STROKE),
            half_plane(center, Vec2::NEG_X),
        ),
    );
    let stem = rounded_rect(
        Rect::new(
            center.x - QUESTION_STROKE * 0.5,
            center.y + 3.0,
            center.x + QUESTION_STROKE * 0.5,
            22.0,
        ),
        QUESTION_STROKE * 0.5,
    );
    let dot = ellipse(Vec2::new(center.x, 26.8), Vec2::splat(3.2));
    raster.shape(
        scaled(union(union(arc, stem), dot), SCALE as f32),
        solid([1.0; 4]),
    );
    raster.finish(images)
}

/// A frame with a rim at the top and the bottom, so the item colour still
/// reads through the middle.
fn metal_band(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ITEM_W, ITEM_H));
    let bounds = Rect::new(0.0, 0.0, ITEM_W, ITEM_H);
    let rims = union(
        rect(Rect::new(0.0, 0.0, ITEM_W, BAND_RIM_H)),
        rect(Rect::new(0.0, ITEM_H - BAND_RIM_H, ITEM_W, ITEM_H)),
    );
    let metal = union(
        outline(rounded_rect(bounds, BAND_RADIUS), BAND_STROKE),
        rims,
    );
    raster.shape(
        scaled(
            intersect(metal, rounded_rect(bounds, BAND_RADIUS)),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

fn metal_lid(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(LID_W, LID_H));
    raster.shape(
        scaled(
            rounded_rect(Rect::new(0.0, 0.0, LID_W, LID_H), LID_H * 0.5),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.shape(
        scaled(
            rounded_rect(Rect::new(7.0, 3.0, LID_W - 7.0, 6.5), 1.75),
            SCALE as f32,
        ),
        solid([1.0, 1.0, 1.0, 0.5]),
    );
    raster.finish(images)
}

/// Left edge, right edge, top and tone of the rock mass and of each boulder
/// resting on it, as fractions of the sprite, plus how far the boulder's peak
/// rises above its shoulders.
const ROCK_BOULDERS: [(f32, f32, f32, f32, f32); 5] = [
    (0.00, 1.00, 0.58, 0.82, 0.00),
    (0.00, 0.34, 0.30, 0.95, 0.10),
    (0.20, 0.56, 0.12, 0.74, 0.08),
    (0.42, 0.78, 0.34, 1.00, 0.09),
    (0.66, 1.00, 0.20, 0.88, 0.09),
];

fn rock_boulder(index: usize) -> impl Fn(Vec2) -> f32 {
    let (left, right, top, _, rise) = ROCK_BOULDERS[index];
    let slab = rounded_rect(
        Rect::new(ROCK_W * left, ROCK_H * top, ROCK_W * right, ROCK_H),
        4.0,
    );
    let peak = triangle(
        Vec2::new(ROCK_W * (left + right) * 0.5, ROCK_H * (top - rise)),
        Vec2::new(ROCK_W * left + 1.0, ROCK_H * (top + 0.20)),
        Vec2::new(ROCK_W * right - 1.0, ROCK_H * (top + 0.20)),
    );
    move |point| slab(point).min(peak(point))
}

fn rock_boulders() -> [impl Fn(Vec2) -> f32; ROCK_BOULDERS.len()] {
    std::array::from_fn(rock_boulder)
}

/// Each boulder gets its own tone and the crease where two of them meet is
/// darkened, so the silhouette reads as separate rocks rather than one slab.
fn rock_base(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ROCK_W, ROCK_H));
    let size = raster.size();
    let outline = rock_boulders();
    raster.shape(
        scaled(
            move |point| {
                outline
                    .iter()
                    .fold(f32::MAX, |near, rock| near.min(rock(point)))
            },
            SCALE as f32,
        ),
        in_pixels(size, {
            let rocks = rock_boulders();
            move |point| {
                let point = point / SCALE as f32;
                let (mut nearest, mut second, mut index) = (f32::MAX, f32::MAX, 0);
                for (candidate, rock) in rocks.iter().enumerate() {
                    let distance = rock(point);
                    if distance < nearest {
                        second = nearest;
                        nearest = distance;
                        index = candidate;
                    } else if distance < second {
                        second = distance;
                    }
                }
                let crease = ((second - nearest) / 1.2).clamp(0.0, 1.0);
                let level = ROCK_BOULDERS[index].3
                    * (1.0 - 0.20 * (point.y / ROCK_H))
                    * (0.78 + 0.22 * crease);
                [level, level, level, 1.0]
            }
        }),
    );
    raster.finish(images)
}

fn rope(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ROPE_W, ROPE_H));
    raster.shape(
        scaled(
            rounded_rect(Rect::new(0.0, 0.0, ROPE_W, ROPE_H), ROPE_H * 0.5),
            SCALE as f32,
        ),
        |uv| {
            let twist = ((uv.x * 9.0 + uv.y * 1.6) * PI).sin() * 0.5 + 0.5;
            let round = 1.0 - 0.3 * uv.y * uv.y;
            let level = ((0.55 + 0.45 * twist) * round).clamp(0.0, 1.0);
            [level, level, level, 1.0]
        },
    );
    raster.finish(images)
}

const PLUG_CAP_H: f32 = 9.0;
const PLUG_WAIST: f32 = 5.0;

fn plug(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(PLUG_W, PLUG_H));
    let cap = rounded_rect(Rect::new(0.0, 0.0, PLUG_W, PLUG_CAP_H), 4.0);
    let shoulder = Vec2::new(2.0, PLUG_CAP_H - 2.0);
    let foot = Vec2::new(PLUG_WAIST, PLUG_H);
    let taper = union(
        triangle(
            shoulder,
            Vec2::new(PLUG_W - shoulder.x, shoulder.y),
            Vec2::new(PLUG_W - foot.x, foot.y),
        ),
        triangle(
            shoulder,
            Vec2::new(PLUG_W - foot.x, foot.y),
            Vec2::new(foot.x, foot.y),
        ),
    );
    raster.shape(
        scaled(smooth_union(cap, taper, 2.0), SCALE as f32),
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

fn rounded(images: &mut Assets<Image>, size: Vec2, radius: f32) -> Handle<Image> {
    let mut raster = raster(size);
    raster.shape(
        scaled(
            rounded_rect(Rect::from_corners(Vec2::ZERO, size), radius),
            SCALE as f32,
        ),
        solid([1.0, 1.0, 1.0, 1.0]),
    );
    raster.finish(images)
}

fn plate(images: &mut Assets<Image>, size: Vec2, radius: f32, border: f32) -> Plate {
    Plate {
        outer: rounded(images, size, radius),
        inner: rounded(images, size - Vec2::splat(2.0 * border), radius - border),
        size,
        border,
    }
}

const ARC_RADIUS: f32 = 11.5;
const ARC_WIDTH: f32 = 5.0;
const HEAD_REACH: f32 = 10.0;
const HEAD_HALF: f32 = 7.5;
const HEAD_BACK: f32 = 2.5;

/// A counter-clockwise arrow: a ring left open in the top-left quadrant, with
/// the head sitting on the upper end of the arc.
fn nav_undo(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::splat(NAV_GLYPH_SIZE));
    let center = Vec2::new(NAV_GLYPH_SIZE * 0.5, NAV_GLYPH_SIZE * 0.5 + 1.0);
    let top = center.y - ARC_RADIUS;
    let arc = union(
        intersect(
            ring(center, ARC_RADIUS, ARC_WIDTH),
            half_plane(center, Vec2::NEG_Y),
        ),
        intersect(
            ring(center, ARC_RADIUS, ARC_WIDTH),
            half_plane(center, Vec2::NEG_X),
        ),
    );
    let head = triangle(
        Vec2::new(center.x - HEAD_REACH, top),
        Vec2::new(center.x + HEAD_BACK, top - HEAD_HALF),
        Vec2::new(center.x + HEAD_BACK, top + HEAD_HALF),
    );
    raster.shape(scaled(union(arc, head), SCALE as f32), solid([1.0; 4]));
    raster.finish(images)
}

fn nav_next(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::splat(NAV_GLYPH_SIZE));
    let inset = NAV_GLYPH_SIZE * 0.22;
    raster.shape(
        scaled(
            triangle(
                Vec2::new(inset, inset),
                Vec2::new(inset, NAV_GLYPH_SIZE - inset),
                Vec2::new(NAV_GLYPH_SIZE - inset, NAV_GLYPH_SIZE * 0.5),
            ),
            SCALE as f32,
        ),
        solid([1.0; 4]),
    );
    raster.finish(images)
}
