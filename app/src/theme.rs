#![allow(dead_code)]

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

pub const ICE_FILL: Color = Color::srgba_u8(0x6F, 0xD8, 0xFF, 140);
pub const ICE_EDGE: Color = Color::srgb_u8(0xC9, 0xF2, 0xFF);

pub const CURTAIN_CLOTH: Color = Color::srgb_u8(0xD6, 0x15, 0x4F);
pub const CURTAIN_TRIM: Color = Color::srgb_u8(0xF0, 0xB4, 0x29);
pub const CURTAIN_ROLL: Color = Color::srgb_u8(0xB8, 0x10, 0x3F);
pub const COLOR_CURTAIN: Color = Color::srgb_u8(0xF3, 0xC7, 0xC4);
pub const COLOR_CURTAIN_SHADE: Color = Color::srgb_u8(0xD8, 0x9E, 0x9C);

pub const SAFE_DOOR: Color = Color::srgb_u8(0x1E, 0x52, 0xC4);
pub const SAFE_PLATE: Color = Color::srgb_u8(0x2D, 0x6B, 0xE0);
pub const SAFE_BOLT: Color = Color::srgb_u8(0xDC, 0x42, 0x36);
pub const SAFE_DIAL: Color = Color::srgb_u8(0xAB, 0xCF, 0xF6);
pub const SAFE_TEXT: Color = Color::srgb_u8(0xFF, 0xFF, 0xFF);

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
pub const Z_GLASS_FRONT: f32 = 2.0;
