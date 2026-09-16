#![allow(dead_code)]

use std::f32::consts::PI;

use bevy::prelude::*;
use water_sort_core::state::COLOR_COUNT;

const MAX_COLOR_BITS: usize = 4;
const MAX_COLOR_COUNT: usize = 1 << MAX_COLOR_BITS;

const _: () = assert!(COLOR_COUNT <= MAX_COLOR_COUNT);

const ITEM_COLORS_ALL: [Color; MAX_COLOR_COUNT] = [
    Color::srgb_u8(0xFF, 0x00, 0xFF),
    Color::srgb_u8(0xF0, 0x47, 0x3E),
    Color::srgb_u8(0xFF, 0x8A, 0x1E),
    Color::srgb_u8(0xFF, 0xC6, 0x1A),
    Color::srgb_u8(0xA6, 0xDB, 0x2B),
    Color::srgb_u8(0x2C, 0xC4, 0x6B),
    Color::srgb_u8(0x14, 0xBF, 0xAE),
    Color::srgb_u8(0x35, 0xB7, 0xF2),
    Color::srgb_u8(0x2F, 0x5B, 0xE0),
    Color::srgb_u8(0x7A, 0x46, 0xE8),
    Color::srgb_u8(0xC0, 0x49, 0xEC),
    Color::srgb_u8(0xFF, 0x63, 0xC4),
    Color::srgb_u8(0xC2, 0x1E, 0x5C),
    Color::srgb_u8(0x9A, 0x5A, 0x34),
    Color::srgb_u8(0x5A, 0x6E, 0x9E),
    Color::srgb_u8(0xED, 0xE4, 0xCF),
];

pub fn item_colors() -> &'static [Color] {
    &ITEM_COLORS_ALL[..COLOR_COUNT]
}

pub fn item_color(color: u8) -> Color {
    debug_assert_ne!(color, 0);
    item_colors()[usize::from(color)]
}

pub const LOCK_COLORS: [Color; 6] = [
    Color::srgb_u8(0x35, 0xC2, 0xFF),
    Color::srgb_u8(0x7C, 0xE0, 0x4A),
    Color::srgb_u8(0xFF, 0xA2, 0x2E),
    Color::srgb_u8(0xFF, 0x67, 0x96),
    Color::srgb_u8(0xB9, 0x8B, 0xFF),
    Color::srgb_u8(0xFF, 0xE0, 0x4D),
];

pub const BACKGROUND_BASE: Color = Color::srgb_u8(0x16, 0x0B, 0x36);
pub const BACKGROUND_GLOW: Color = Color::srgb_u8(0x33, 0x1A, 0x72);
pub const BACKGROUND_EDGE: Color = Color::srgb_u8(0x0B, 0x06, 0x20);
pub const STAR: Color = Color::srgb_u8(0xFF, 0xFF, 0xFF);

pub const GLASS_RIM: Color = Color::srgba_u8(0x7F, 0xB4, 0xE8, 140);
pub const GLASS_INTERIOR: Color = Color::srgba_u8(0x0D, 0x13, 0x30, 89);
pub const GLASS_HIGHLIGHT: Color = Color::srgba_u8(0xFF, 0xFF, 0xFF, 46);

pub const SELECT_GLOW: Color = Color::srgb_u8(0xC3, 0x9B, 0xEE);

pub const ITEM_HIDDEN: Color = Color::srgb_u8(0x0A, 0x0A, 0x12);
pub const ITEM_QUESTION: Color = Color::srgb_u8(0xB9, 0xA7, 0xE8);

pub const CORK_TOP: Color = Color::srgb_u8(0xE9, 0xB3, 0x3F);
pub const CORK_BODY: Color = Color::srgb_u8(0xC4, 0x84, 0x2A);
pub const CORK_SHADE: Color = Color::srgb_u8(0x8B, 0x5A, 0x22);
pub const ROPE: Color = Color::srgb_u8(0xB4, 0x76, 0x3A);
pub const PLUG: Color = Color::srgb_u8(0x8E, 0x5A, 0x2C);

pub const METAL_LIGHT: Color = Color::srgb_u8(0xB9, 0xC6, 0xD6);
pub const METAL_DARK: Color = Color::srgb_u8(0x6B, 0x7A, 0x8C);
pub const METAL_BAND: Color = Color::srgb_u8(0x93, 0xA3, 0xB5);

pub const ROCK_LIGHT: Color = Color::srgb_u8(0xD8, 0xD2, 0xC4);
pub const ROCK_SHADE: Color = Color::srgb_u8(0xA7, 0x9E, 0x8C);

pub const ICE_FILL: Color = Color::srgba_u8(0x7C, 0xE4, 0xFF, 235);
pub const ICE_EDGE: Color = Color::srgb_u8(0xC9, 0xF2, 0xFF);
pub const ICE_BASE: Color = Color::srgba_u8(0x5C, 0xD2, 0xFF, 245);
pub const ICE_FLASH: Color = Color::srgb_u8(0xFF, 0xFF, 0xFF);

pub const CURTAIN_CLOTH: Color = Color::srgb_u8(0xD6, 0x15, 0x4F);
pub const CURTAIN_TRIM: Color = Color::srgb_u8(0xF0, 0xB4, 0x29);
pub const CURTAIN_ROLL: Color = Color::srgb_u8(0xB8, 0x10, 0x3F);
pub const COLOR_CURTAIN: Color = Color::srgb_u8(0xF3, 0xC7, 0xC4);
pub const COLOR_CURTAIN_SHADE: Color = Color::srgb_u8(0xD8, 0x9E, 0x9C);

pub const SAFE_DOOR: Color = Color::srgb_u8(0x13, 0x30, 0x7C);
pub const SAFE_PLATE: Color = Color::srgb_u8(0x2D, 0x6B, 0xE0);
pub const SAFE_BOLT: Color = Color::srgb_u8(0xDC, 0x42, 0x36);
pub const SAFE_DIAL: Color = Color::srgb_u8(0xAB, 0xCF, 0xF6);
pub const SAFE_TEXT: Color = Color::srgb_u8(0xFF, 0xFF, 0xFF);
pub const SAFE_SHADOW: Color = Color::srgb_u8(0x10, 0x2A, 0x66);

pub const WOOD_PLANK: Color = Color::srgb_u8(0xE8, 0xA9, 0x60);
pub const WOOD_SHADE: Color = Color::srgb_u8(0xCE, 0x8C, 0x43);
pub const WOOD_FRAME: Color = Color::srgb_u8(0xD4, 0x79, 0x1E);
pub const WOOD_FRAME_EDGE: Color = Color::srgb_u8(0xF0, 0xB8, 0x4E);

pub const HEADER_FILL: Color = Color::srgb_u8(0x0A, 0x0F, 0x3E);
pub const HEADER_BORDER: Color = Color::srgb_u8(0x7B, 0x3F, 0xE4);
pub const HEADER_TEXT: Color = Color::srgb_u8(0xFC, 0xF5, 0xDB);
pub const NAV_FILL: Color = Color::srgb_u8(0x1E, 0x21, 0x8A);
pub const NAV_BORDER: Color = Color::srgb_u8(0x3A, 0x3F, 0xC0);
pub const NAV_BTN: Color = Color::srgb_u8(0x78, 0x34, 0xD9);
pub const NAV_RING: Color = Color::srgb_u8(0xC3, 0x9B, 0xEE);
pub const NAV_RING_IN: Color = Color::srgb_u8(0xE8, 0xB2, 0x4A);
pub const NAV_GLYPH: Color = Color::srgb_u8(0xFF, 0xFF, 0xFF);
pub const NAV_BTN_OFF: Color = Color::srgb_u8(0x3B, 0x34, 0x70);
pub const NAV_GLYPH_OFF: Color = Color::srgb_u8(0x8A, 0x85, 0xB5);
pub const OVERLAY_DIM: Color = Color::srgba_u8(0x0B, 0x06, 0x20, 140);
pub const BANNER_TEXT: Color = Color::srgb_u8(0xFF, 0xD8, 0x4D);
pub const BANNER_SHADOW: Color = Color::srgb_u8(0x6B, 0x2E, 0x00);

pub const CANVAS_W: f32 = 440.0;
pub const CANVAS_H: f32 = 956.0;

pub const ITEM_H: f32 = 34.0;
pub const BOTTLE_W: f32 = 62.0;
pub const GLASS_WALL: f32 = 5.0;
pub const COL_GAP: f32 = 8.0;
pub const NECK_H: f32 = 26.0;
pub const BASE_H: f32 = 8.0;

pub const INTRO_SLIDE: f32 = 0.32;
pub const INTRO_STAGGER: f32 = 0.035;
pub const SELECT_LIFT: f32 = 0.16;
pub const SELECT_RISE: f32 = 26.0;
pub const POUR_TRAVEL: f32 = 0.26;
pub const POUR_TILT: f32 = 0.20;
pub const POUR_PER_ITEM: f32 = 0.11;
pub const POUR_STREAM_MIN: f32 = 0.14;
pub const POUR_RETURN: f32 = 0.24;
pub const FINALIZE_SWIRL: f32 = 0.50;
pub const CORK_DROP: f32 = 0.22;
pub const REVEAL_ITEM: f32 = 0.18;
pub const LID_OPEN: f32 = 0.18;
pub const PLUG_TOGGLE: f32 = 0.30;
pub const ICE_SHATTER: f32 = 0.55;
pub const CURTAIN_LIFT: f32 = 0.35;
pub const COLOR_CURTAIN_LIFT: f32 = 0.40;
pub const SAFE_TICK: f32 = 0.22;
pub const SAFE_OPEN: f32 = 0.45;
pub const KEY_FLIGHT: f32 = 0.45;
pub const DOOR_OPEN: f32 = 0.50;
pub const COMPLETE_DIM: f32 = 0.25;
pub const COMPLETE_TEXT: f32 = 0.45;
pub const CONFETTI_LIFE: f32 = 2.2;
pub const NAV_SHAKE: f32 = 0.28;
/// Range decorations resolve while the finalize swirl is still climbing, so the
/// ice, the curtain and the safes react to the cork rather than to the pour.
pub const RANGE_DELAY: f32 = FINALIZE_SWIRL;

pub const STAR_COUNT: usize = 40;
pub const STAR_SIZE_MIN: f32 = 7.0;
pub const STAR_SIZE_MAX: f32 = 16.0;
pub const STAR_PERIOD_MIN: f32 = 1.8;
pub const STAR_PERIOD_MAX: f32 = 4.5;
pub const STAR_POP_INTERVAL: f32 = 2.5;
pub const STAR_POP_TIME: f32 = 0.4;
pub const STAR_POP_SCALE: f32 = 1.8;

pub const GLOW_SIZE: f32 = 820.0;
pub const GLOW_ALPHA: f32 = 0.55;
pub const VIGNETTE_ALPHA: f32 = 0.85;

pub const Z_BACKGROUND: f32 = -100.0;
pub const Z_GLOW: f32 = -99.0;
pub const Z_VIGNETTE: f32 = -98.0;
pub const Z_STARS: f32 = -97.0;
pub const Z_BOARD: f32 = 0.0;
pub const Z_GLASS_BACK: f32 = 0.0;
pub const Z_ITEM: f32 = 1.0;
pub const Z_ITEM_SURFACE: f32 = 0.1;
pub const Z_ITEM_BAND: f32 = 0.2;
pub const Z_ITEM_LID: f32 = 0.3;
pub const Z_ITEM_QUESTION: f32 = 0.4;
pub const Z_GLASS_FRONT: f32 = 2.0;
pub const Z_ROCKS: f32 = 2.4;
pub const Z_ICE_BASE: f32 = 3.2;
pub const Z_ICE_FROST: f32 = 3.4;
pub const Z_ICE_FRAME: f32 = 3.5;
pub const Z_CURTAIN: f32 = 5.0;
pub const Z_CURTAIN_TRIM: f32 = 5.1;
pub const Z_CURTAIN_ROLL: f32 = 5.2;
pub const Z_SAFE: f32 = 6.0;
pub const Z_SAFE_FACE: f32 = 0.1;
pub const Z_SAFE_CROSS: f32 = 0.2;
pub const Z_SAFE_DIAL: f32 = 0.3;
pub const Z_SAFE_TEXT: f32 = 0.4;
pub const Z_ROPE: f32 = 2.6;
pub const Z_PLUG: f32 = 2.8;
pub const Z_BOTTLE: f32 = 0.0;
pub const Z_HALO: f32 = -1.0;
pub const Z_CORK: f32 = 3.0;
pub const Z_STREAM: f32 = 15.0;
pub const Z_POUR: f32 = 20.0;
pub const Z_SWIRL: f32 = 25.0;
pub const Z_OVERLAY: f32 = 40.0;
pub const Z_CONFETTI: f32 = 45.0;
pub const Z_HUD: f32 = 50.0;
pub const Z_HUD_FACE: f32 = 0.1;
pub const Z_HUD_GLYPH: f32 = 0.2;
pub const Z_BANNER: f32 = 55.0;

pub const HALO_ALPHA: f32 = 0.45;
pub const HALO_WIDTH: f32 = BOTTLE_W * 2.6;
pub const HALO_MARGIN: f32 = 46.0;
pub const HALO_FADE: f32 = 12.0;

pub const LIFT_STIFFNESS: f32 = 420.0;
pub const LIFT_DAMPING: f32 = 26.0;

pub const FLUID_STIFFNESS: f32 = 90.0;
pub const FLUID_DAMPING: f32 = 9.0;
pub const FLUID_MAX_TILT: f32 = 0.22;
/// A liquid surface counter-rotates against the glass to stay level, but a
/// fully tipped bottle would push the surface slab outside the silhouette.
pub const SURFACE_MAX_TILT: f32 = 0.32;
pub const FLUID_DRIVE: f32 = 0.0016;
pub const FLUID_DRIVE_MAX: f32 = 6.0;
pub const FLUID_LAND_KICK: f32 = 3.5;
pub const FLUID_POUR_KICK: f32 = 1.8;

pub const POUR_ARC: f32 = 46.0;
pub const POUR_HOVER_X: f32 = BOTTLE_W * 0.55;
pub const POUR_HOVER_Y: f32 = NECK_H * 1.4;
pub const POUR_ANGLE_FULL: f32 = 60.0 * PI / 180.0;
pub const POUR_ANGLE_EMPTY: f32 = 100.0 * PI / 180.0;

pub const STREAM_DROPLETS: usize = 10;
pub const STREAM_DROPLET_W: f32 = 11.0;
pub const STREAM_DROPLET_H: f32 = 15.0;
pub const STREAM_CYCLE: f32 = 0.16;
pub const STREAM_ARC: f32 = 14.0;

pub const SPLASH_COUNT: usize = 4;
pub const SPLASH_LIFE: f32 = 0.28;
pub const SPLASH_SPEED: f32 = 130.0;
pub const SPLASH_GRAVITY: f32 = -900.0;

pub const SWIRL_STARS: usize = 14;
pub const SWIRL_TURNS: f32 = 1.6;
pub const SWIRL_RADIUS: f32 = BOTTLE_W * 0.32;
pub const SWIRL_STAGGER: f32 = 0.02;
pub const SWIRL_SIZE: f32 = 26.0;
pub const SWIRL_TINT: f32 = 0.8;

pub const CORK_W: f32 = 34.0;
pub const CORK_H: f32 = 26.0;
pub const CORK_CAP_H: f32 = 10.0;
pub const CORK_RISE: f32 = 30.0;
pub const CORK_SQUASH: f32 = 1.15;

pub const QUESTION_W: f32 = 22.0;
pub const QUESTION_H: f32 = 30.0;

pub const BAND_RADIUS: f32 = 6.0;
pub const BAND_STROKE: f32 = 3.5;
pub const BAND_RIM_H: f32 = 5.0;

pub const LID_W: f32 = 58.0;
pub const LID_H: f32 = 15.0;
pub const LID_ANGLE: f32 = -96.0 * PI / 180.0;
pub const LID_RATE: f32 = 14.0;

pub const ROCK_W: f32 = BOTTLE_W + 14.0;
pub const ROCK_H: f32 = 44.0;
pub const ROCK_DROP: f32 = 9.0;

pub const ROPE_W: f32 = 32.0;
pub const ROPE_H: f32 = 12.0;
pub const ROPE_DROP: f32 = 14.0;

pub const PLUG_W: f32 = 24.0;
pub const PLUG_H: f32 = 26.0;
pub const PLUG_SEAT: Vec2 = Vec2::new(0.0, -3.0);
pub const PLUG_HANG: Vec2 = Vec2::new(-23.0, -25.0);
pub const PLUG_HANG_ANGLE: f32 = -0.7;
pub const PLUG_ARC: f32 = 12.0;

pub const DROPLET_W: f32 = 12.0;
pub const DROPLET_H: f32 = 16.0;

pub const INTRO_ENTRY_MARGIN: f32 = 120.0;

pub const HEADER_PILL: Vec2 = Vec2::new(214.0, 58.0);
pub const HEADER_PILL_RADIUS: f32 = 29.0;
pub const HEADER_PILL_BORDER: f32 = 4.0;
pub const HEADER_FONT: f32 = 30.0;

pub const NAV_PANEL: Vec2 = Vec2::new(244.0, 100.0);
pub const NAV_PANEL_RADIUS: f32 = 50.0;
pub const NAV_PANEL_BORDER: f32 = 4.0;
pub const NAV_BUTTON: f32 = 74.0;
pub const NAV_BUTTON_RADIUS: f32 = 25.0;
pub const NAV_RING_W: f32 = 5.0;
pub const NAV_RING_IN_W: f32 = 3.0;
pub const NAV_GLYPH_SIZE: f32 = 36.0;
pub const NAV_GAP: f32 = 30.0;

pub const NEXT_PILL: Vec2 = Vec2::new(236.0, 66.0);
pub const NEXT_PILL_RADIUS: f32 = 33.0;
pub const NEXT_PILL_BORDER: f32 = 4.0;
pub const NEXT_FONT: f32 = 26.0;
pub const NEXT_OFFSET: f32 = -84.0;
pub const NEXT_GLYPH_SIZE: f32 = 28.0;
pub const NEXT_GLYPH_X: f32 = 92.0;

pub const BANNER_FONT: f32 = 38.0;
pub const BANNER_OFFSET: f32 = 26.0;
pub const BANNER_LINE: f32 = 1.25;
pub const BANNER_SCALE_FROM: f32 = 0.4;
pub const BANNER_SWAY: f32 = 2.0 * PI / 180.0;
pub const BANNER_SWAY_HZ: f32 = 0.6;
pub const BANNER_SHADOW_OFFSET: Vec2 = Vec2::new(0.0, -4.0);

pub const CONFETTI_COUNT: usize = 90;
pub const CONFETTI_W: f32 = 9.0;
pub const CONFETTI_H: f32 = 13.0;
pub const CONFETTI_SPREAD_X: f32 = 60.0;
pub const CONFETTI_SPEED_MIN: f32 = 40.0;
pub const CONFETTI_SPEED_MAX: f32 = 140.0;
pub const CONFETTI_DROP: f32 = 280.0;
pub const CONFETTI_GRAVITY: f32 = -300.0;
pub const CONFETTI_SPIN: f32 = 6.0;

pub const FLUID_NAV_KICK: f32 = 2.5;

pub const ICE_MARGIN: f32 = COL_GAP / 2.0;
pub const ICE_BASE_H: f32 = 50.0;
pub const ICE_CROWN_H: f32 = 38.0;
pub const ICE_FRAME_W: f32 = 4.0;
pub const ICE_FRAME_ALPHA: f32 = 0.5;
pub const ICE_BURST: f32 = 0.09;
pub const ICE_FLASH_TIME: f32 = 0.16;
pub const ICE_SHARDS: usize = 16;
pub const ICE_SHARD_W: f32 = 13.0;
pub const ICE_SHARD_H: f32 = 16.0;
pub const ICE_SHARD_SPEED: f32 = 200.0;
pub const ICE_SHARD_GRAVITY: f32 = -520.0;
pub const ICE_SHARD_SPIN: f32 = 7.0;
pub const ICE_SHARD_LIFE: f32 = 0.7;

pub const CURTAIN_MARGIN: f32 = COL_GAP / 2.0;
pub const CURTAIN_ROLL_W: f32 = 27.0;
pub const CURTAIN_TRIM_H: f32 = 11.0;
pub const CURTAIN_KNOB: f32 = 18.0;
pub const CURTAIN_WAVE: f32 = 0.06;
pub const CURTAIN_WAVE_HZ: f32 = 2.5;

pub const SAFE_MARGIN: f32 = 5.0;
pub const SAFE_RADIUS: f32 = 20.0;
pub const SAFE_BORDER: f32 = 8.0;
pub const SAFE_CROSS_D: f32 = 74.0;
pub const SAFE_DIAL_D: f32 = 44.0;
pub const SAFE_HUB_D: f32 = 32.0;
pub const SAFE_FONT: f32 = 26.0;
pub const SAFE_SPIN: f32 = -120.0 * PI / 180.0;
pub const SAFE_POP: f32 = 0.34;
pub const SAFE_SWING: f32 = -13.0 * PI / 180.0;
pub const SAFE_SHUT: f32 = 0.94;

pub const DOOR_MARGIN: f32 = COL_GAP / 2.0;
pub const DOOR_RADIUS: f32 = 14.0;
pub const DOOR_BORDER: f32 = 7.0;
pub const DOOR_GROOVE_W: f32 = 30.0;
pub const DOOR_SWING: f32 = 11.0 * PI / 180.0;
pub const DOOR_SHUT: f32 = 0.93;
pub const DOOR_GROOVE_ALPHA: f32 = 0.55;

pub const LOCK_W: f32 = 40.0;
pub const LOCK_H: f32 = 34.0;
pub const LOCK_SHACKLE_W: f32 = 26.0;
pub const LOCK_SHACKLE_H: f32 = 24.0;
pub const LOCK_SHACKLE_POP: f32 = 9.0;
pub const LOCK_DROP: f32 = 210.0;
pub const LOCK_TIP: f32 = 22.0 * PI / 180.0;
pub const LOCK_PLATE: Color = Color::srgb_u8(0x2B, 0x1E, 0x4A);

pub const KEY_W: f32 = 34.0;
pub const KEY_H: f32 = 16.0;
pub const KEY_BADGE: f32 = 0.62;
pub const KEY_ARC: f32 = 70.0;
pub const KEY_SPIN: f32 = 2.5;
pub const KEY_SPARKS: usize = 8;
pub const KEY_SPARK_SIZE: f32 = 15.0;
pub const KEY_SPARK_LIFE: f32 = 0.4;
pub const KEY_SPARK_SPEED: f32 = 55.0;
pub const KEY_SPARK_GRAVITY: f32 = -120.0;
/// The lock is only unbolted once the key has arrived.
pub const LOCK_DELAY: f32 = KEY_FLIGHT;

pub const TAG_W: f32 = 26.0;
pub const TAG_H: f32 = 32.0;
pub const TAG_CORD_W: f32 = 4.0;
pub const TAG_CORD_H: f32 = 16.0;
pub const TAG_HANG: Vec2 = Vec2::new(28.0, -13.0);
pub const TAG_SWAY: f32 = 3.0 * PI / 180.0;
pub const TAG_SWAY_HZ: f32 = 0.4;
pub const TAG_SWING: f32 = 1.4;
pub const TAG_MAX_SWING: f32 = 0.5;

pub const COLOR_CURTAIN_MARGIN: f32 = COL_GAP / 2.0;
pub const COLOR_CURTAIN_STRIPS: usize = 6;
pub const COLOR_CURTAIN_RIPPLE: f32 = 0.09;
pub const COLOR_CURTAIN_DRIFT: f32 = 20.0;
pub const COLOR_CURTAIN_ICON_W: f32 = 30.0;
pub const COLOR_CURTAIN_ICON_H: f32 = 44.0;
pub const COLOR_CURTAIN_ICON_Y: f32 = 0.30;
pub const COLOR_CURTAIN_RING: f32 = 1.35;

pub const Z_COLOR_CURTAIN: f32 = 5.6;
pub const Z_COLOR_CURTAIN_ICON: f32 = 5.7;
pub const Z_DOOR: f32 = 7.0;
pub const Z_DOOR_LOCK: f32 = 7.4;
pub const Z_KEY: f32 = 7.6;
pub const Z_TAG_CORD: f32 = 2.5;
pub const Z_TAG: f32 = 2.7;
pub const Z_ITEM_KEY: f32 = 0.5;
