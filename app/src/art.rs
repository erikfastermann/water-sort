use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, SpriteImageMode, TextureSlicer};

use crate::geometry::{COL_PITCH, COLOR_CURTAIN_STRIP, CURTAIN_H, DOOR_MIN, SAFE_MIN, bottle_h};
use crate::raster::{
    Raster, ellipse, half_plane, in_pixels, intersect, outline, radial, radial_inverse, rect, ring,
    rotated, rounded_rect, scaled, smooth_union, solid, sparkle, triangle, union,
};
use crate::rng::Pcg32;
use crate::theme::{
    BAND_RADIUS, BAND_RIM_H, BAND_STROKE, BASE_H, BOTTLE_W, COLOR_CURTAIN_ICON_H,
    COLOR_CURTAIN_ICON_W, CONFETTI_H, CONFETTI_W, CORK_CAP_H, CORK_H, CORK_W, CURTAIN_KNOB,
    CURTAIN_ROLL_W, DOOR_BORDER, DOOR_GROOVE_W, DOOR_RADIUS, DROPLET_H, DROPLET_W, GLASS_WALL,
    HEADER_PILL, HEADER_PILL_BORDER, HEADER_PILL_RADIUS, ICE_BASE_H, ICE_CROWN_H, ICE_SHARD_H,
    ICE_SHARD_W, ITEM_H, KEY_H, KEY_W, LID_H, LID_W, LOCK_H, LOCK_SHACKLE_H, LOCK_SHACKLE_W,
    LOCK_W, NAV_BUTTON, NAV_BUTTON_RADIUS, NAV_GLYPH_SIZE, NAV_PANEL, NAV_PANEL_BORDER,
    NAV_PANEL_RADIUS, NECK_H, NEXT_PILL, NEXT_PILL_BORDER, NEXT_PILL_RADIUS, PLUG_H, PLUG_W,
    QUESTION_H, QUESTION_W, ROCK_H, ROCK_W, ROPE_H, ROPE_W, SAFE_BORDER, SAFE_CROSS_D, SAFE_DIAL_D,
    SAFE_HUB_D, SAFE_RADIUS, TAG_H, TAG_W,
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
    pub cork: Handle<Image>,
    pub cork_cap: Handle<Image>,
    pub question: Handle<Image>,
    pub metal_band: Handle<Image>,
    pub metal_lid: Handle<Image>,
    pub rock_base: Handle<Image>,
    pub rope: Handle<Image>,
    pub plug: Handle<Image>,
    pub droplet: Handle<Image>,
    pub ice_base: Handle<Image>,
    pub ice_crown: Handle<Image>,
    pub ice_shard: Handle<Image>,
    pub curtain_cloth: Handle<Image>,
    pub curtain_roll: Handle<Image>,
    pub knob: Handle<Image>,
    pub safe_door: Plate,
    pub safe_cross: Handle<Image>,
    pub safe_dial: Handle<Image>,
    pub safe_hub: Handle<Image>,
    pub door_plate: Plate,
    pub door_grooves: Handle<Image>,
    pub lock_body: Handle<Image>,
    pub lock_shackle: Handle<Image>,
    pub key: Handle<Image>,
    pub tag: Handle<Image>,
    pub color_cloth: Handle<Image>,
    pub bottle_icon: Handle<Image>,
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
        app.init_resource::<GlassArt>()
            .add_systems(PreStartup, build_art);
    }
}

/// A bottle is drawn at its own capacity, so the glass comes in as many sizes
/// as there are capacities. Rasterizing all of them up front costs more than
/// the whole rest of the art, so each one is built the first time it is used.
#[derive(Resource, Default)]
pub struct GlassArt(HashMap<u8, (Handle<Image>, Handle<Image>)>);

impl GlassArt {
    pub fn get(
        &mut self,
        images: &mut Assets<Image>,
        capacity: u8,
    ) -> (Handle<Image>, Handle<Image>) {
        self.0
            .entry(capacity)
            .or_insert_with(|| (glass_back(images, capacity), glass_front(images, capacity)))
            .clone()
    }
}

pub fn glass_size(capacity: u8) -> Vec2 {
    Vec2::new(BOTTLE_W, bottle_h(capacity)) + Vec2::splat(2.0 * GLASS_PAD)
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
        cork: cork(&mut images),
        cork_cap: cork_cap(&mut images),
        question: question(&mut images),
        metal_band: metal_band(&mut images),
        metal_lid: metal_lid(&mut images),
        rock_base: rock_base(&mut images),
        rope: rope(&mut images),
        plug: plug(&mut images),
        droplet: droplet(&mut images),
        ice_base: ice_base(&mut images),
        ice_crown: ice_crown(&mut images),
        ice_shard: ice_shard(&mut images),
        curtain_cloth: curtain_cloth(&mut images),
        curtain_roll: curtain_roll(&mut images),
        knob: knob(&mut images),
        safe_door: plate(&mut images, SAFE_MIN, SAFE_RADIUS, SAFE_BORDER),
        safe_cross: safe_cross(&mut images),
        safe_dial: safe_dial(&mut images),
        safe_hub: rounded(&mut images, Vec2::splat(SAFE_HUB_D), SAFE_HUB_D * 0.5),
        door_plate: plate(&mut images, DOOR_MIN, DOOR_RADIUS, DOOR_BORDER),
        door_grooves: door_grooves(&mut images),
        lock_body: lock_body(&mut images),
        lock_shackle: lock_shackle(&mut images),
        key: key(&mut images),
        tag: tag(&mut images),
        color_cloth: color_cloth(&mut images),
        bottle_icon: bottle_icon(&mut images),
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

/// Nine slicing keeps corners and borders at their authored size while the
/// middle stretches, which is what lets one safe door cover any span. The
/// corner cap is `1 / SCALE` because the art is rasterized at `SCALE` and drawn
/// in design units.
pub fn sliced(inset: f32) -> SpriteImageMode {
    SpriteImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(inset * SCALE as f32),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0 / SCALE as f32,
    })
}

/// Repeats the texture horizontally every authored width instead of stretching
/// it, so one sprite can cover a span of any size with an even pattern.
pub fn tiled() -> SpriteImageMode {
    SpriteImageMode::Tiled {
        tile_x: true,
        tile_y: false,
        stretch_value: 1.0 / SCALE as f32,
    }
}

/// The left `fraction` of a texture authored at `design` units, for a sprite
/// that reveals its art instead of squashing it.
pub fn crop(design: Vec2, fraction: f32) -> Rect {
    let size = design * SCALE as f32;
    Rect::new(0.0, 0.0, size.x * fraction, size.y)
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

fn bottle_sdf(capacity: u8) -> impl Fn(Vec2) -> f32 {
    let height = bottle_h(capacity);
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

fn glass_back(images: &mut Assets<Image>, capacity: u8) -> Handle<Image> {
    let size = glass_size(capacity);
    let mut raster = raster(size);
    let pixels = raster.size();
    raster.shape(
        scaled(bottle_sdf(capacity), SCALE as f32),
        in_pixels(pixels, {
            let sdf = scaled(bottle_sdf(capacity), SCALE as f32);
            move |point| {
                let inside = (-sdf(point) / (GLASS_WALL * 2.0 * SCALE as f32)).clamp(0.0, 1.0);
                [1.0, 1.0, 1.0, 1.0f32.lerp(0.5, inside)]
            }
        }),
    );
    raster.finish(images)
}

fn glass_front(images: &mut Assets<Image>, capacity: u8) -> Handle<Image> {
    let height = bottle_h(capacity);
    let size = glass_size(capacity);
    let center = GLASS_PAD + BOTTLE_W * 0.5;
    let mut raster = raster(size);

    raster.shape(
        scaled(outline(bottle_sdf(capacity), GLASS_STROKE), SCALE as f32),
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

/// Facet bands that repeat exactly four times across a column, so neighbouring
/// tiles of one range join without a seam.
fn ice_bands(u: f32, v: f32) -> f32 {
    let band = (u * 4.0 + v * 0.4).fract();
    0.80 + 0.20 * (band * PI).sin().powf(0.6)
}

fn ice_base(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(COL_PITCH, ICE_BASE_H));
    let size = raster.size();
    let bounds = raster.bounds();
    raster.shape(
        rect(bounds),
        in_pixels(size, |point| {
            let point = point / SCALE as f32;
            let (u, v) = (point.x / COL_PITCH, point.y / ICE_BASE_H);
            let cap = (1.0 - v * 7.0).clamp(0.0, 1.0);
            let level = (ice_bands(u, v) * (1.0 - 0.32 * v) + 0.42 * cap).clamp(0.0, 1.0);
            [level, level, level, 1.0]
        }),
    );
    raster.finish(images)
}

const ICICLES: [(f32, f32, f32); 4] = [
    (0.16, 4.5, 18.0),
    (0.38, 6.0, 29.0),
    (0.62, 5.0, 22.0),
    (0.85, 5.5, 33.0),
];

const ICE_RAIL: f32 = 11.0;

/// The top edge of a frozen run: a solid rail with icicles hanging off it. The
/// bottles below stay unobscured, which is what keeps their colours readable.
fn ice_crown(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(COL_PITCH, ICE_CROWN_H));
    let size = raster.size();
    let teeth = std::array::from_fn::<_, { ICICLES.len() }, _>(|index| {
        let (u, half, len) = ICICLES[index];
        triangle(
            Vec2::new(u * COL_PITCH - half, 0.0),
            Vec2::new(u * COL_PITCH + half, 0.0),
            Vec2::new(u * COL_PITCH, len),
        )
    });
    raster.shape(
        scaled(
            move |point| {
                teeth.iter().fold(
                    rect(Rect::new(0.0, 0.0, COL_PITCH, ICE_RAIL))(point),
                    |near, tooth| near.min(tooth(point)),
                )
            },
            SCALE as f32,
        ),
        in_pixels(size, |point| {
            let point = point / SCALE as f32;
            let (u, v) = (point.x / COL_PITCH, point.y / ICE_CROWN_H);
            let level = ice_bands(u, v);
            [level, level, level, 1.0 - 0.35 * v]
        }),
    );
    raster.finish(images)
}

fn ice_shard(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(ICE_SHARD_W, ICE_SHARD_H));
    raster.shape(
        scaled(
            triangle(
                Vec2::new(ICE_SHARD_W * 0.5, 0.0),
                Vec2::new(0.0, ICE_SHARD_H),
                Vec2::new(ICE_SHARD_W, ICE_SHARD_H * 0.72),
            ),
            SCALE as f32,
        ),
        |uv| {
            let level = 0.70 + 0.30 * (1.0 - uv.y);
            [level, level, level, 1.0]
        },
    );
    raster.finish(images)
}

fn curtain_cloth(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(COL_PITCH, CURTAIN_H));
    let size = raster.size();
    let bounds = raster.bounds();
    raster.shape(
        rect(bounds),
        in_pixels(size, |point| {
            let point = point / SCALE as f32;
            let (u, v) = (point.x / COL_PITCH, point.y / CURTAIN_H);
            let fold = (u * 4.0 * TAU).sin() * 0.5 + 0.5;
            let ends = 1.0 - 0.30 * (v * 2.0 - 1.0).abs().powi(3);
            let level = ((0.58 + 0.42 * fold) * ends).clamp(0.0, 1.0);
            [level, level, level, 1.0]
        }),
    );
    raster.finish(images)
}

fn curtain_roll(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(CURTAIN_ROLL_W, CURTAIN_H));
    raster.shape(
        scaled(
            rounded_rect(
                Rect::new(0.0, 0.0, CURTAIN_ROLL_W, CURTAIN_H),
                CURTAIN_ROLL_W * 0.5,
            ),
            SCALE as f32,
        ),
        |uv| {
            let round = (1.0 - ((uv.x + 0.28) / 1.15).powi(2)).max(0.0);
            let level = (0.42 + 0.68 * round).clamp(0.0, 1.0);
            [level, level, level, 1.0]
        },
    );
    raster.finish(images)
}

fn knob(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::splat(CURTAIN_KNOB));
    let half = raster.size() * 0.5;
    raster.shape(ellipse(half, half), |uv| {
        let lit = (uv - Vec2::new(-0.35, -0.35)).length() / 1.7;
        let level = (1.1 - lit).clamp(0.0, 1.0);
        [level, level, level, 1.0]
    });
    raster.finish(images)
}

const SAFE_ARM: f32 = 0.10;
const SAFE_ARM_REACH: f32 = 0.40;

fn safe_cross(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::splat(SAFE_CROSS_D));
    let center = Vec2::splat(SAFE_CROSS_D * 0.5);
    let thickness = SAFE_CROSS_D * SAFE_ARM;
    let reach = SAFE_CROSS_D * SAFE_ARM_REACH;
    let bar = |horizontal: bool| {
        let span = Vec2::new(
            if horizontal { reach } else { thickness },
            if horizontal { thickness } else { reach },
        );
        rounded_rect(Rect::from_center_size(center, span * 2.0), thickness * 0.9)
    };
    let arms = union(bar(true), bar(false));
    let caps = std::array::from_fn::<_, 4, _>(|index| {
        let angle = index as f32 * TAU / 4.0;
        ellipse(
            center + Vec2::new(angle.cos(), angle.sin()) * reach,
            Vec2::splat(thickness * 1.35),
        )
    });
    raster.shape(
        scaled(
            rotated(
                move |point| {
                    caps.iter()
                        .fold(arms(point), |near, cap| near.min(cap(point)))
                },
                center,
                PI * 0.25,
            ),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

fn safe_dial(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::splat(SAFE_DIAL_D));
    let center = Vec2::splat(SAFE_DIAL_D * 0.5);
    let radius = SAFE_DIAL_D * 0.40;
    let notch = SAFE_DIAL_D * 0.055;
    let band = ring(center, radius, SAFE_DIAL_D * 0.10);
    raster.shape(
        scaled(
            move |point| {
                (0..8).fold(band(point), |near, index| {
                    let angle = index as f32 * TAU / 8.0;
                    let at = center + Vec2::new(angle.cos(), angle.sin()) * radius;
                    near.min((point - at).length() - notch)
                })
            },
            SCALE as f32,
        ),
        item_shade,
    );
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

const GROOVE_H: f32 = 8.0;
const GROOVE_W: f32 = 2.2;

/// One plank seam, repeated across a door half by [`tiled`].
fn door_grooves(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(DOOR_GROOVE_W, GROOVE_H));
    raster.shape(
        scaled(rect(Rect::new(0.0, 0.0, GROOVE_W, GROOVE_H)), SCALE as f32),
        solid([1.0; 4]),
    );
    raster.finish(images)
}

const LOCK_RADIUS: f32 = 9.0;
const KEYHOLE_LEVEL: f32 = 0.34;

fn lock_body(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(LOCK_W, LOCK_H));
    raster.shape(
        scaled(
            rounded_rect(Rect::new(0.0, 0.0, LOCK_W, LOCK_H), LOCK_RADIUS),
            SCALE as f32,
        ),
        item_shade,
    );
    let eye = Vec2::new(LOCK_W * 0.5, LOCK_H * 0.38);
    let slot = triangle(
        Vec2::new(eye.x - 2.0, eye.y),
        Vec2::new(eye.x + 2.0, eye.y),
        Vec2::new(eye.x, LOCK_H * 0.78),
    );
    let keyhole = union(ellipse(eye, Vec2::splat(5.0)), slot);
    raster.shape(
        scaled(keyhole, SCALE as f32),
        solid([KEYHOLE_LEVEL, KEYHOLE_LEVEL, KEYHOLE_LEVEL, 1.0]),
    );
    raster.finish(images)
}

fn lock_shackle(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(LOCK_SHACKLE_W, LOCK_SHACKLE_H));
    let center = Vec2::new(LOCK_SHACKLE_W * 0.5, LOCK_SHACKLE_H);
    let radius = LOCK_SHACKLE_W * 0.5 - 3.0;
    raster.shape(
        scaled(
            intersect(ring(center, radius, 6.0), half_plane(center, Vec2::Y)),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

const KEY_BOW: f32 = 6.5;
const KEY_SHAFT: f32 = 3.4;

/// Bow on the left, shaft to the right, two teeth hanging off the tip.
fn key(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(KEY_W, KEY_H));
    let bow = Vec2::new(KEY_BOW + 1.0, KEY_H * 0.5);
    let shaft = rounded_rect(
        Rect::new(
            bow.x,
            KEY_H * 0.5 - KEY_SHAFT * 0.5,
            KEY_W - 1.0,
            KEY_H * 0.5 + KEY_SHAFT * 0.5,
        ),
        KEY_SHAFT * 0.5,
    );
    let tooth = |at: f32, drop: f32| rect(Rect::new(at, KEY_H * 0.5, at + 3.0, KEY_H * 0.5 + drop));
    let teeth = union(tooth(KEY_W - 8.0, 5.0), tooth(KEY_W - 14.0, 3.5));
    raster.shape(
        scaled(
            union(ring(bow, KEY_BOW, 3.6), union(shaft, teeth)),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}

const TAG_RADIUS: f32 = 7.0;
const TAG_HOLE: f32 = 4.0;
const TAG_HOLE_LEVEL: f32 = 0.42;

fn tag(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(Vec2::new(TAG_W, TAG_H));
    let hole = Vec2::new(TAG_W * 0.5, 8.0);
    raster.shape(
        scaled(
            rounded_rect(Rect::new(0.0, 0.0, TAG_W, TAG_H), TAG_RADIUS),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.shape(
        scaled(ellipse(hole, Vec2::splat(TAG_HOLE)), SCALE as f32),
        solid([TAG_HOLE_LEVEL, TAG_HOLE_LEVEL, TAG_HOLE_LEVEL, 1.0]),
    );
    raster.finish(images)
}

/// One vertical strip of a colour curtain. The fold pattern is exactly one
/// period wide, so neighbouring strips join without a seam.
fn color_cloth(images: &mut Assets<Image>) -> Handle<Image> {
    let mut raster = raster(COLOR_CURTAIN_STRIP);
    let size = raster.size();
    let bounds = raster.bounds();
    raster.shape(
        rect(bounds),
        in_pixels(size, |point| {
            let point = point / SCALE as f32;
            let (u, v) = (
                point.x / COLOR_CURTAIN_STRIP.x,
                point.y / COLOR_CURTAIN_STRIP.y,
            );
            let fold = (u * TAU).sin() * 0.5 + 0.5;
            let rail = if (0.05..=0.95).contains(&v) {
                1.0
            } else {
                0.74
            };
            let level = ((0.60 + 0.40 * fold) * rail).clamp(0.0, 1.0);
            [level, level, level, 1.0]
        }),
    );
    raster.finish(images)
}

/// A filled bottle silhouette, used as the colour badge on a colour curtain.
fn bottle_icon(images: &mut Assets<Image>) -> Handle<Image> {
    let (w, h) = (COLOR_CURTAIN_ICON_W, COLOR_CURTAIN_ICON_H);
    let mut raster = raster(Vec2::new(w, h));
    let body = rounded_rect(Rect::new(0.0, h * 0.30, w, h), w * 0.28);
    let neck = rounded_rect(Rect::new(w * 0.34, h * 0.08, w * 0.66, h * 0.42), w * 0.10);
    let rim = rounded_rect(Rect::new(w * 0.26, 0.0, w * 0.74, h * 0.12), w * 0.08);
    raster.shape(
        scaled(
            smooth_union(smooth_union(body, neck, w * 0.22), rim, 1.5),
            SCALE as f32,
        ),
        item_shade,
    );
    raster.finish(images)
}
