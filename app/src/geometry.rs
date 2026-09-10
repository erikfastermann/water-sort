#![allow(dead_code)]

use std::range::Range;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use water_sort_core::layout::Layout;

use crate::safe;
use crate::theme::{
    BASE_H, BOTTLE_W, CANVAS_H, CANVAS_W, COL_GAP, COLOR_CURTAIN_MARGIN, CURTAIN_MARGIN,
    DOOR_MARGIN, GLASS_WALL, ICE_MARGIN, INTRO_ENTRY_MARGIN, INTRO_STAGGER, ITEM_H, NECK_H,
    SAFE_MARGIN,
};

const CAP: [u8; Layout::LINES as usize] = Layout::MAX_CAPACITY;

const LINE_STEP: u8 = CAP[1] - CAP[0];

const _: () = assert!(CAP[1] - CAP[0] == CAP[2] - CAP[1]);

pub const LINES: usize = Layout::LINES as usize;

pub const COL_PITCH: f32 = BOTTLE_W + COL_GAP;
pub const LINE_PITCH: f32 = LINE_STEP as f32 * ITEM_H;

pub const BOARD_W: f32 = Layout::COLUMNS as f32 * COL_PITCH - COL_GAP;
pub const BOARD_H: f32 = (Layout::LINES - 1) as f32 * LINE_PITCH + outer_h(1);

pub const HEADER_H: f32 = 96.0;
pub const NAV_H: f32 = 150.0;

pub const CANVAS_TOP: f32 = CANVAS_H / 2.0;
pub const CANVAS_BOTTOM: f32 = -CANVAS_TOP;
pub const HEADER_BOTTOM: f32 = CANVAS_TOP - HEADER_H;
pub const NAV_TOP: f32 = CANVAS_BOTTOM + NAV_H;
pub const PLAY_TOP: f32 = HEADER_BOTTOM;
pub const PLAY_BOTTOM: f32 = NAV_TOP;
pub const PLAY_CENTER: Vec2 = Vec2::new(0.0, (PLAY_TOP + PLAY_BOTTOM) / 2.0);
pub const HEADER_CENTER: Vec2 = Vec2::new(0.0, CANVAS_TOP - HEADER_H / 2.0);
pub const NAV_CENTER: Vec2 = Vec2::new(0.0, CANVAS_BOTTOM + NAV_H / 2.0);

const _: () = assert!(BOARD_W <= CANVAS_W);
const _: () = assert!(BOARD_H <= PLAY_TOP - PLAY_BOTTOM);

/// How far the two chrome bands may grow into the play area before the tallest
/// board stops fitting between them.
pub const BAND_SLACK: f32 = PLAY_TOP - PLAY_BOTTOM - BOARD_H;

/// Re-reading the insets forces a style recalculation on the web, so it is
/// polled rather than sampled every frame.
const SAFE_POLL: f32 = 0.25;

pub struct GeometryPlugin;

impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bands>().add_systems(
            Update,
            (
                track_safe_area,
                place_anchored.run_if(resource_changed::<Bands>),
            )
                .chain(),
        );
    }
}

/// The three horizontal bands of the screen, after the camera notch and the
/// home indicator have eaten into the two chrome ones. Every value is the
/// centre of that band; without insets they are the design constants above.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct Bands {
    pub header: Vec2,
    pub nav: Vec2,
    pub play: Vec2,
}

impl Default for Bands {
    fn default() -> Self {
        Self::inset(0.0, 0.0)
    }
}

impl Bands {
    /// Insets are in world units and grow the header and nav bands inward. The
    /// board keeps priority: an inset pair larger than [`BAND_SLACK`] is scaled
    /// down rather than allowed to squeeze the play area.
    pub fn inset(top: f32, bottom: f32) -> Self {
        let top = top.max(0.0);
        let bottom = bottom.max(0.0);
        let total = top + bottom;
        let scale = if total > BAND_SLACK {
            BAND_SLACK / total
        } else {
            1.0
        };

        let header_bottom = HEADER_BOTTOM - top * scale;
        let nav_top = NAV_TOP + bottom * scale;
        Self {
            header: Vec2::new(0.0, header_bottom + HEADER_H / 2.0),
            nav: Vec2::new(0.0, nav_top - NAV_H / 2.0),
            play: Vec2::new(0.0, (header_bottom + nav_top) / 2.0),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Band {
    Header,
    Nav,
    Play,
}

/// Chrome that follows a band instead of sitting at a fixed world position.
#[derive(Component, Clone, Copy)]
pub struct Anchored {
    band: Band,
    offset: Vec2,
}

impl Anchored {
    pub fn new(band: Band, offset: Vec2) -> Self {
        Self { band, offset }
    }

    fn center(self, bands: &Bands) -> Vec2 {
        let band = match self.band {
            Band::Header => bands.header,
            Band::Nav => bands.nav,
            Band::Play => bands.play,
        };
        band + self.offset
    }
}

fn track_safe_area(
    time: Res<Time>,
    camera: Query<&Projection, With<Camera2d>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut bands: ResMut<Bands>,
    mut clock: Local<f32>,
) {
    *clock -= time.delta_secs();
    if *clock > 0.0 {
        return;
    }
    *clock = SAFE_POLL;

    let Ok(Projection::Orthographic(projection)) = camera.single() else {
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };
    let height = window.height();
    if height <= 0.0 {
        return;
    }

    let area = projection.area.height();
    let per_pixel = area / height;
    // A screen taller than the design rect already shows dead space above and
    // below it, and that absorbs the inset before the bands have to.
    let overhang = (area - CANVAS_H) / 2.0;
    let raw = safe::insets();
    let next = Bands::inset(raw.x * per_pixel - overhang, raw.y * per_pixel - overhang);

    if *bands != next {
        *bands = next;
    }
}

fn place_anchored(bands: Res<Bands>, mut anchored: Query<(&Anchored, &mut Transform)>) {
    for (anchor, mut transform) in &mut anchored {
        let center = anchor.center(&bands);
        transform.translation.x = center.x;
        transform.translation.y = center.y;
    }
}

pub const fn max_capacity(lines: u8) -> u8 {
    CAP[lines as usize - 1]
}

pub const fn outer_h(lines: u8) -> f32 {
    max_capacity(lines) as f32 * ITEM_H + NECK_H + BASE_H
}

pub const MOUTH_INSET: f32 = 6.0;

/// Core validates frozen, curtain and lock ranges to be single line, so every
/// range decoration is exactly this tall.
pub const ICE_H: f32 = outer_h(1) + 2.0 * ICE_MARGIN;
pub const CURTAIN_H: f32 = outer_h(1) + 2.0 * CURTAIN_MARGIN;

pub const DOOR_H: f32 = outer_h(1) + 2.0 * DOOR_MARGIN;

/// A door is split in the middle and each half is nine sliced, so the art has
/// to be authored at the narrowest half a single-bottle group can produce.
pub const DOOR_MIN: Vec2 = Vec2::new((BOTTLE_W + 2.0 * DOOR_MARGIN) / 2.0, DOOR_H);

/// One strip of a colour curtain, authored at the height of a single-line
/// bottle; a taller bottle stretches the folds vertically.
pub const COLOR_CURTAIN_STRIP: Vec2 = Vec2::new(
    (BOTTLE_W + 2.0 * COLOR_CURTAIN_MARGIN) / crate::theme::COLOR_CURTAIN_STRIPS as f32,
    outer_h(1) + 2.0 * COLOR_CURTAIN_MARGIN,
);

/// The smallest door a safe can need, and therefore the size its art is drawn
/// at; every real door is at least this large in both axes.
pub const SAFE_MIN: Vec2 = Vec2::new(BOTTLE_W + 2.0 * SAFE_MARGIN, outer_h(1) + 2.0 * SAFE_MARGIN);

/// How far off screen a bottle starts its intro slide.
pub const INTRO_ENTRY: f32 = BOARD_W + INTRO_ENTRY_MARGIN;

/// Stagger of the last possible repr column, so the intro phase knows when the
/// slowest bottle has landed.
pub const MAX_INTRO_DELAY: f32 = (Layout::REPR_LINE_LEN - 1) as f32 * INTRO_STAGGER;

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct BoardGeometry {
    origin: Vec2,
}

impl BoardGeometry {
    pub fn new(bands: Bands, slots: impl Iterator<Item = (Range<u8>, u8)>) -> Self {
        let mut used: Option<Rect> = None;
        for (lines, repr_column) in slots {
            let rect = Self { origin: Vec2::ZERO }.bottle_rect(lines, repr_column);
            used = Some(match used {
                Some(bounds) => bounds.union(rect),
                None => rect,
            });
        }

        let center = used.map_or(Vec2::ZERO, |bounds| bounds.center());
        Self {
            origin: bands.play - center,
        }
    }

    /// Core centres every layout line inside `REPR_LINE_LEN`, so a line holding
    /// an odd number of columns fewer than the full grid sits on half-pitch
    /// repr columns. The repr column is therefore a half-column index, not a
    /// grid column times two.
    pub fn bottle_rect(&self, lines: Range<u8>, repr_column: u8) -> Rect {
        let span = lines.end - lines.start;
        let left = self.origin.x + f32::from(repr_column) * COL_PITCH * 0.5;
        let top = self.origin.y - f32::from(lines.start) * LINE_PITCH;
        Rect::new(left, top - outer_h(span), left + BOTTLE_W, top)
    }

    pub fn interior_rect(&self, bottle: Rect) -> Rect {
        Rect::new(
            bottle.min.x + GLASS_WALL,
            bottle.min.y + BASE_H,
            bottle.max.x - GLASS_WALL,
            bottle.max.y - NECK_H,
        )
    }

    pub fn item_rect(&self, bottle: Rect, index: u8) -> Rect {
        let interior = self.interior_rect(bottle);
        let bottom = interior.min.y + f32::from(index) * ITEM_H;
        Rect::new(interior.min.x, bottom, interior.max.x, bottom + ITEM_H)
    }

    pub fn mouth(&self, bottle: Rect) -> Vec2 {
        Vec2::new(bottle.center().x, bottle.max.y - MOUTH_INSET)
    }
}

pub fn check_capacity(capacity: u8, lines: u8) {
    debug_assert!(capacity <= max_capacity(lines));
}

#[cfg(test)]
mod tests {
    use std::range::Range;

    use super::{
        BOARD_H, BOARD_W, Bands, BoardGeometry, COL_PITCH, LINE_PITCH, PLAY_CENTER, outer_h,
    };
    use crate::theme::{BOTTLE_W, ITEM_H};

    fn slot(start: u8, end: u8, repr_column: u8) -> (Range<u8>, u8) {
        (Range { start, end }, repr_column)
    }

    #[test]
    fn bands_without_insets_are_the_design_constants() {
        let bands = Bands::default();
        assert_eq!(bands.header, super::HEADER_CENTER);
        assert_eq!(bands.nav, super::NAV_CENTER);
        assert_eq!(bands.play, PLAY_CENTER);
    }

    #[test]
    fn insets_move_the_bands_inward_and_recentre_the_play_area() {
        let bands = Bands::inset(60.0, 40.0);
        assert_eq!(bands.header.y, super::HEADER_CENTER.y - 60.0);
        assert_eq!(bands.nav.y, super::NAV_CENTER.y + 40.0);
        assert_eq!(bands.play.y, PLAY_CENTER.y - 10.0);
    }

    #[test]
    fn the_board_keeps_priority_over_an_oversized_inset() {
        let bands = Bands::inset(super::BAND_SLACK, super::BAND_SLACK);
        let play = (bands.header.y - super::HEADER_H / 2.0) - (bands.nav.y + super::NAV_H / 2.0);
        assert!((play - BOARD_H).abs() < 1e-3, "play area shrank to {play}");
    }

    #[test]
    fn derived_constants() {
        assert_eq!(COL_PITCH, 70.0);
        assert_eq!(LINE_PITCH, 204.0);
        assert_eq!(outer_h(1), 170.0);
        assert_eq!(outer_h(2), 374.0);
        assert_eq!(outer_h(3), 578.0);
        assert_eq!(BOARD_W, 412.0);
        assert_eq!(BOARD_H, 578.0);
    }

    #[test]
    fn full_board_is_centered() {
        let slots: Vec<_> = (0..3)
            .flat_map(|line| (0..6).map(move |column| slot(line, line + 1, column * 2)))
            .collect();
        let geometry = BoardGeometry::new(Bands::default(), slots.iter().copied());

        let first = geometry.bottle_rect(slots[0].0, slots[0].1);
        let last = geometry.bottle_rect(slots[17].0, slots[17].1);
        let bounds = first.union(last);

        assert_eq!(bounds.width(), BOARD_W);
        assert_eq!(bounds.center(), PLAY_CENTER);
    }

    #[test]
    fn single_line_level_is_centered() {
        let slots = [slot(1, 2, 4), slot(1, 2, 6)];
        let geometry = BoardGeometry::new(Bands::default(), slots.iter().copied());
        let bounds = geometry
            .bottle_rect(slots[0].0, slots[0].1)
            .union(geometry.bottle_rect(slots[1].0, slots[1].1));

        assert_eq!(bounds.center(), PLAY_CENTER);
        assert_eq!(bounds.width(), COL_PITCH + BOTTLE_W);
    }

    #[test]
    fn shorter_lines_sit_on_half_pitch_columns() {
        let slots = [
            slot(0, 1, 4),
            slot(0, 1, 6),
            slot(1, 2, 3),
            slot(1, 2, 5),
            slot(1, 2, 7),
        ];
        let geometry = BoardGeometry::new(Bands::default(), slots.iter().copied());
        let wide = geometry.bottle_rect(slots[0].0, slots[0].1);
        let narrow = geometry.bottle_rect(slots[2].0, slots[2].1);

        assert_eq!(wide.min.x - narrow.min.x, COL_PITCH / 2.0);
        assert_eq!(
            geometry.bottle_rect(slots[3].0, slots[3].1).center().x,
            PLAY_CENTER.x
        );
    }

    #[test]
    fn items_stack_from_the_interior_bottom() {
        let geometry = BoardGeometry::new(Bands::default(), core::iter::empty());
        let bottle = geometry.bottle_rect(Range { start: 0, end: 1 }, 0);
        let interior = geometry.interior_rect(bottle);

        let first = geometry.item_rect(bottle, 0);
        assert_eq!(first.min.y, interior.min.y);
        assert_eq!(first.height(), ITEM_H);

        let top = geometry.item_rect(bottle, 3);
        assert_eq!(top.max.y, interior.max.y);
        assert_eq!(first.width(), interior.width());
    }

    #[test]
    fn multi_line_bottles_share_the_item_grid() {
        let geometry = BoardGeometry::new(Bands::default(), core::iter::empty());
        let short = geometry.bottle_rect(Range { start: 1, end: 2 }, 0);
        let tall = geometry.bottle_rect(Range { start: 0, end: 2 }, 2);

        for index in 0..4 {
            assert_eq!(
                geometry.item_rect(short, index).min.y,
                geometry.item_rect(tall, index).min.y
            );
        }
        assert_eq!(geometry.item_rect(short, 3).max.y, short.max.y - 26.0);
        assert_eq!(geometry.item_rect(tall, 9).max.y, tall.max.y - 26.0);
    }
}
