use std::f32::consts::{PI, TAU};

use bevy::ecs::system::SystemParam;
use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2d};

use crate::anim::Flow;
use crate::art::{Art, CURTAIN_SAG, CURTAIN_SHEET, crop, sliced, tiled};
use crate::fx::Particle;
use crate::geometry::{BoardGeometry, COL_PITCH};
use crate::rng::Pcg32;
use crate::theme;
use crate::view::{BoardView, BottleView, MovePlan};

#[derive(Resource)]
pub struct DecorRng(pub Pcg32);

#[derive(Component)]
pub struct IceGroup {
    first: u8,
    bounds: Rect,
    shattered: bool,
}

#[derive(Component)]
pub struct IcePart {
    first: u8,
    tint: Color,
}

#[derive(Component)]
pub struct Curtain {
    first: u8,
    last: u8,
    left: f32,
    height: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CurtainRole {
    Sheet(u8),
    Roll,
}

#[derive(Component)]
pub struct CurtainPart {
    first: u8,
    role: CurtainRole,
}

#[derive(Component)]
pub struct Safe {
    first: u8,
    members: Vec<u8>,
}

#[derive(Component)]
pub struct SafePart {
    first: u8,
    tint: Color,
    dial: bool,
}

#[derive(Component)]
pub struct SafeCounter {
    first: u8,
}

/// Bottles hidden behind one door. Safe counters are decremented in lockstep by
/// core, so equal counters stay equal and unequal ones stay unequal: the
/// grouping computed here at level start can never change while the level runs.
pub fn safe_groups(view: &BoardView) -> Vec<Vec<u8>> {
    let mut safes: Vec<&BottleView> = view
        .bottles
        .iter()
        .filter(|bottle| bottle.safe_counter > 0)
        .collect();
    safes.sort_by_key(|bottle| (bottle.lines.start, bottle.repr_column));

    let mut groups: Vec<Vec<&BottleView>> = Vec::new();
    for bottle in safes {
        let joins = groups
            .last()
            .and_then(|group| group.last())
            .is_some_and(|previous| {
                previous.line_span() == 1
                    && bottle.line_span() == 1
                    && previous.lines.start == bottle.lines.start
                    && previous.repr_column + 2 == bottle.repr_column
                    && previous.safe_counter == bottle.safe_counter
            });
        match joins {
            true => groups.last_mut().expect("a group to join").push(bottle),
            false => groups.push(vec![bottle]),
        }
    }

    groups
        .into_iter()
        .map(|group| group.into_iter().map(|bottle| bottle.id).collect())
        .collect()
}

pub fn spawn(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    view: &BoardView,
    colors: &LockColors,
) {
    for range in &view.frozen_ranges {
        spawn_ice(
            commands,
            board,
            art,
            geometry,
            view,
            range.start,
            range.last,
        );
    }
    for range in &view.curtain_ranges {
        spawn_curtain(
            commands,
            board,
            art,
            geometry,
            view,
            range.start,
            range.last,
        );
    }
    for members in safe_groups(view) {
        spawn_safe(commands, board, art, geometry, view, members);
    }
    for (key, range) in &view.lock_groups {
        spawn_door(
            commands,
            board,
            art,
            geometry,
            view,
            colors.get(*key),
            range.start,
            range.last,
        );
    }
    for bottle in &view.bottles {
        if let Some(color) = bottle.color_curtain {
            spawn_color_curtain(commands, board, art, geometry, bottle, color);
        }
    }
}

fn bottle_rect(geometry: &BoardGeometry, view: &BoardView, id: u8) -> Rect {
    geometry.bottle_rect(view.get(id).slot())
}

fn span_rect(
    geometry: &BoardGeometry,
    view: &BoardView,
    ids: impl Iterator<Item = u8>,
) -> Option<Rect> {
    ids.map(|id| bottle_rect(geometry, view, id))
        .reduce(|span, rect| span.union(rect))
}

fn spawn_ice(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    view: &BoardView,
    first: u8,
    last: u8,
) {
    let Some(span) = span_rect(geometry, view, first..=last) else {
        return;
    };
    let wide = span.inflate(theme::DECOR_MARGIN);
    let block = Rect::new(
        wide.min.x,
        wide.min.y,
        wide.max.x,
        wide.min.y + theme::ICE_BASE_H,
    );
    let center = block.center();

    let group = commands
        .spawn((
            IceGroup {
                first,
                bounds: block,
                shattered: false,
            },
            Transform::from_translation(center.extend(0.0)),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();

    for id in first..=last {
        let column = bottle_rect(geometry, view, id).center().x - center.x;
        commands.spawn((
            IcePart {
                first,
                tint: theme::ICE_EDGE,
            },
            Sprite {
                image: art.ice_base.clone(),
                color: theme::ICE_EDGE,
                custom_size: Some(Vec2::new(COL_PITCH, theme::ICE_BASE_H)),
                ..default()
            },
            Transform::from_xyz(column, 0.0, theme::Z_ICE_BASE),
            ChildOf(group),
        ));
    }
}

fn spawn_curtain(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    view: &BoardView,
    first: u8,
    last: u8,
) {
    let Some(span) = span_rect(geometry, view, first..=last) else {
        return;
    };
    let bounds = span.inflate(theme::DECOR_MARGIN);
    let center = bounds.center();
    let half = bounds.half_size();
    let left = -half.x;
    let height = bounds.height();

    let curtain = commands
        .spawn((
            Curtain {
                first,
                last,
                left,
                height,
            },
            Transform::from_translation(center.extend(0.0)),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();

    for column in 0..=last - first {
        commands.spawn((
            CurtainPart {
                first,
                role: CurtainRole::Sheet(column),
            },
            Sprite {
                image: if column == 0 {
                    art.curtain_end.clone()
                } else {
                    art.curtain_cloth.clone()
                },
                custom_size: Some(Vec2::new(COL_PITCH, height + 2.0 * CURTAIN_SAG)),
                ..default()
            },
            Anchor::CENTER_LEFT,
            Transform::from_xyz(left + f32::from(column) * COL_PITCH, 0.0, theme::Z_CURTAIN),
            ChildOf(curtain),
        ));
    }

    let roll = commands
        .spawn((
            CurtainPart {
                first,
                role: CurtainRole::Roll,
            },
            Sprite {
                image: art.curtain_roll.clone(),
                color: theme::CURTAIN_ROLL,
                custom_size: Some(Vec2::new(theme::CURTAIN_ROLL_W, height + 2.0 * CURTAIN_SAG)),
                ..default()
            },
            Transform::from_xyz(
                left - theme::CURTAIN_ROLL_W * 0.5,
                0.0,
                theme::Z_CURTAIN_ROLL,
            ),
            ChildOf(curtain),
        ))
        .id();
    for side in [1.0, -1.0] {
        commands.spawn((
            Sprite {
                image: art.knob.clone(),
                color: theme::CURTAIN_TRIM,
                custom_size: Some(Vec2::splat(theme::CURTAIN_KNOB)),
                ..default()
            },
            Transform::from_xyz(0.0, side * (half.y + CURTAIN_SAG), 0.1),
            ChildOf(roll),
        ));
    }
}

fn spawn_safe(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    view: &BoardView,
    members: Vec<u8>,
) {
    let Some(span) = span_rect(geometry, view, members.iter().copied()) else {
        return;
    };
    let bounds = span.inflate(theme::DECOR_MARGIN);
    let first = members[0];
    let offset = bounds.width() * 0.5;

    let safe = commands
        .spawn((
            Safe { first, members },
            Transform::from_xyz(bounds.min.x, bounds.center().y, theme::Z_SAFE),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();

    commands.spawn((
        SafePart {
            first,
            tint: theme::SAFE_DOOR,
            dial: false,
        },
        Sprite {
            image: art.safe_door.outer.clone(),
            color: theme::SAFE_DOOR,
            custom_size: Some(bounds.size()),
            image_mode: sliced(theme::SAFE_RADIUS),
            ..default()
        },
        Transform::from_xyz(offset, 0.0, 0.0),
        ChildOf(safe),
    ));
    commands.spawn((
        SafePart {
            first,
            tint: theme::SAFE_PLATE,
            dial: false,
        },
        Sprite {
            image: art.safe_door.inner.clone(),
            color: theme::SAFE_PLATE,
            custom_size: Some(bounds.size() - Vec2::splat(2.0 * theme::SAFE_BORDER)),
            image_mode: sliced(theme::SAFE_RADIUS - theme::SAFE_BORDER),
            ..default()
        },
        Transform::from_xyz(offset, 0.0, theme::Z_SAFE_FACE),
        ChildOf(safe),
    ));

    let dial = commands
        .spawn((
            SafePart {
                first,
                tint: theme::SAFE_BOLT,
                dial: true,
            },
            Sprite {
                image: art.safe_cross.clone(),
                color: theme::SAFE_BOLT,
                custom_size: Some(Vec2::splat(theme::SAFE_CROSS_D)),
                ..default()
            },
            Transform::from_xyz(offset, 0.0, theme::Z_SAFE_CROSS),
            ChildOf(safe),
        ))
        .id();
    commands.spawn((
        SafePart {
            first,
            tint: theme::SAFE_DIAL,
            dial: false,
        },
        Sprite {
            image: art.safe_dial.clone(),
            color: theme::SAFE_DIAL,
            custom_size: Some(Vec2::splat(theme::SAFE_DIAL_D)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.05),
        ChildOf(dial),
    ));

    commands.spawn((
        SafePart {
            first,
            tint: theme::SAFE_DOOR,
            dial: false,
        },
        Sprite {
            image: art.safe_hub.clone(),
            color: theme::SAFE_DOOR,
            custom_size: Some(Vec2::splat(theme::SAFE_HUB_D)),
            ..default()
        },
        Transform::from_xyz(offset, 0.0, theme::Z_SAFE_DIAL),
        ChildOf(safe),
    ));
    commands.spawn((
        SafeCounter { first },
        Text2d::new(String::new()),
        TextFont {
            font_size: FontSize::Px(theme::SAFE_FONT),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(theme::SAFE_TEXT),
        Transform::from_xyz(offset, 0.0, theme::Z_SAFE_TEXT),
        ChildOf(safe),
    ));
}

/// How far a range effect has run, `0.0` for the whole pour and the swirl that
/// precedes it. `None` when this move does not carry the effect at all.
fn phase(
    effects: Option<(&MovePlan, f32)>,
    active: impl Fn(&MovePlan) -> bool,
    duration: f32,
) -> Option<f32> {
    delayed(effects, active, theme::RANGE_DELAY, duration)
}

fn delayed(
    effects: Option<(&MovePlan, f32)>,
    active: impl Fn(&MovePlan) -> bool,
    delay: f32,
    duration: f32,
) -> Option<f32> {
    effects
        .filter(|(plan, _)| active(plan))
        .map(|(_, clock)| ((clock - delay) / duration).clamp(0.0, 1.0))
}

fn shown(visible: bool) -> Visibility {
    if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

pub fn sync_ice(
    mut commands: Commands,
    art: Res<Art>,
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut rng: ResMut<DecorRng>,
    mut groups: Query<(&mut IceGroup, &mut Transform, &mut Visibility)>,
    mut parts: Query<(&IcePart, &mut Sprite)>,
) {
    let effects = flow.effects();
    let mut states: Vec<(u8, f32, f32)> = Vec::new();

    for (mut group, mut transform, mut visibility) in &mut groups {
        let first = group.first;
        let frozen = view.get(first).frozen;
        let shatter = phase(
            effects,
            |plan| plan.unfreeze.is_some_and(|range| range.start == first),
            theme::ICE_SHATTER,
        );

        if frozen {
            group.shattered = false;
        }
        if let Some(progress) = shatter
            && progress > 0.0
            && !group.shattered
        {
            group.shattered = true;
            burst(&mut commands, &art, &mut rng.0, group.bounds);
        }

        let progress = shatter.unwrap_or(0.0);
        *visibility = shown(frozen || shatter.is_some_and(|value| value < 1.0));
        transform.scale = Vec3::splat(1.0 + theme::ICE_BURST * (progress * PI).sin());

        let flash = shatter.map_or(0.0, |progress| {
            (1.0 - progress / theme::ICE_FLASH_TIME).clamp(0.0, 1.0)
        });
        let alpha = 1.0 - (progress / 0.7).clamp(0.0, 1.0);
        states.push((first, flash, alpha));
    }

    for (part, mut sprite) in &mut parts {
        let Some((_, flash, alpha)) = states.iter().copied().find(|(id, ..)| *id == part.first)
        else {
            continue;
        };
        let tint = part.tint.mix(&theme::ICE_FLASH, flash);
        sprite.color = tint.with_alpha(part.tint.alpha() * alpha);
    }
}

fn burst(commands: &mut Commands, art: &Art, rng: &mut Pcg32, bounds: Rect) {
    for _ in 0..theme::ICE_SHARDS {
        let at = Vec2::new(
            rng.range(bounds.min.x, bounds.max.x),
            rng.range(bounds.min.y, bounds.max.y),
        );
        let away = (at - bounds.center()).normalize_or(Vec2::Y);
        commands.spawn((
            Particle {
                velocity: away * rng.range(0.5, 1.0) * theme::ICE_SHARD_SPEED,
                gravity: theme::ICE_SHARD_GRAVITY,
                spin: rng.range(-theme::ICE_SHARD_SPIN, theme::ICE_SHARD_SPIN),
                life: theme::ICE_SHARD_LIFE,
                max_life: theme::ICE_SHARD_LIFE,
            },
            Sprite {
                image: art.ice_shard.clone(),
                color: theme::ICE_EDGE,
                custom_size: Some(Vec2::new(theme::ICE_SHARD_W, theme::ICE_SHARD_H)),
                ..default()
            },
            Transform::from_translation(at.extend(theme::Z_SWIRL)),
        ));
    }
}

/// Columns of a curtain still covered, counting the one currently rolling away
/// as a fraction, so the sheet, the trim and the roll all read the same number.
fn covered(view: &BoardView, flow: &Flow, first: u8, last: u8) -> f32 {
    let still = (first..=last)
        .filter(|id| view.get(*id).behind_curtain)
        .count() as f32;
    let lifting = phase(
        flow.effects(),
        |plan| {
            plan.curtains_lifted
                .iter()
                .any(|id| (first..=last).contains(id))
        },
        theme::CURTAIN_LIFT,
    );
    still + lifting.map_or(0.0, |progress| 1.0 - ease(progress))
}

fn ease(progress: f32) -> f32 {
    EaseFunction::CubicInOut.sample_clamped(progress)
}

pub fn sync_curtains(
    view: Res<BoardView>,
    flow: Res<Flow>,
    time: Res<Time>,
    curtains: Query<&Curtain>,
    mut parts: Query<(&CurtainPart, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let elapsed = time.elapsed_secs();
    let states: Vec<(u8, f32, f32, f32)> = curtains
        .iter()
        .map(|curtain| {
            (
                curtain.first,
                covered(&view, &flow, curtain.first, curtain.last),
                curtain.left,
                curtain.height,
            )
        })
        .collect();

    for (part, mut sprite, mut transform, mut visibility) in &mut parts {
        let Some((_, width, left, height)) =
            states.iter().copied().find(|(id, ..)| *id == part.first)
        else {
            continue;
        };

        match part.role {
            CurtainRole::Sheet(column) => {
                let fraction = (width - f32::from(column)).clamp(0.0, 1.0);
                *visibility = shown(fraction > 0.0);
                sprite.custom_size =
                    Some(Vec2::new(fraction * COL_PITCH, height + 2.0 * CURTAIN_SAG));
                sprite.rect = Some(crop(CURTAIN_SHEET, fraction));

                // Every column of a sheet has to breathe in step: the hems are
                // part of the cloth now, so a per column phase would break them
                // at the seam.
                let wave = (elapsed * theme::CURTAIN_WAVE_HZ * TAU).sin();
                transform.scale.y = 1.0 + theme::CURTAIN_WAVE * wave * (width.fract() * PI).sin();
            }
            CurtainRole::Roll => {
                *visibility = shown(width > 0.0);
                transform.translation.x = left + width * COL_PITCH - theme::CURTAIN_ROLL_W * 0.5;
            }
        }
    }
}

/// What the door, the dial and the counter all read off one safe.
fn safe_state(view: &BoardView, effects: Option<(&MovePlan, f32)>, safe: &Safe) -> SafeState {
    let member = |ids: &Vec<u8>| ids.iter().any(|id| safe.members.contains(id));
    SafeState {
        counter: view.get(safe.first).safe_counter,
        tick: phase(effects, |plan| member(&plan.safe_ticks), theme::SAFE_TICK),
        open: phase(effects, |plan| member(&plan.safes_opened), theme::SAFE_OPEN),
    }
}

pub fn sync_safes(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut safes: Query<(&Safe, &mut Transform, &mut Visibility)>,
    mut parts: Query<(&SafePart, &mut Sprite, &mut Transform), Without<Safe>>,
) {
    let effects = flow.effects();
    let mut states: Vec<(u8, SafeState)> = Vec::new();

    for (safe, mut transform, mut visibility) in &mut safes {
        let state = safe_state(&view, effects, safe);
        let swing = state.open.map_or(0.0, ease);

        *visibility = shown(state.counter > 0 || state.open.is_some_and(|open| open < 1.0));
        transform.scale.x = 1.0 - theme::SAFE_SHUT * swing;
        transform.rotation = Quat::from_rotation_z(theme::SAFE_SWING * swing);
        states.push((safe.first, state));
    }

    for (part, mut sprite, mut transform) in &mut parts {
        let Some((_, state)) = states.iter().find(|(id, _)| *id == part.first) else {
            continue;
        };
        sprite.color = part.tint.with_alpha(state.alpha());
        if part.dial {
            transform.rotation = Quat::from_rotation_z(theme::SAFE_SPIN * state.turns());
        }
    }
}

pub fn sync_safe_counters(
    view: Res<BoardView>,
    flow: Res<Flow>,
    safes: Query<&Safe>,
    mut counters: Query<(&SafeCounter, &mut Text2d, &mut TextColor, &mut Transform)>,
) {
    let effects = flow.effects();
    for (counter, mut text, mut color, mut transform) in &mut counters {
        let Some(safe) = safes.iter().find(|safe| safe.first == counter.first) else {
            continue;
        };
        let state = safe_state(&view, effects, safe);
        let progress = state.tick.unwrap_or(1.0);

        let wanted = state.shown_counter().to_string();
        if text.as_str() != wanted {
            **text = wanted;
        }
        transform.scale = Vec3::splat(1.0 + theme::SAFE_POP * (progress * PI).sin());
        color.0 = theme::SAFE_TEXT.with_alpha((progress * 2.0 - 1.0).abs() * state.alpha());
    }
}

impl SafeState {
    fn alpha(&self) -> f32 {
        let swing = self.open.map_or(0.0, ease);
        1.0 - ((swing - 0.6) / 0.4).clamp(0.0, 1.0)
    }

    /// The dial rests on a full turn per counted move, so a tick interpolates
    /// between two resting angles instead of snapping back when it ends.
    fn turns(&self) -> f32 {
        f32::from(self.counter) + self.tick.map_or(0.0, |tick| 1.0 - ease(tick))
    }

    fn shown_counter(&self) -> u8 {
        match self.tick {
            Some(tick) if tick < 0.5 => self.counter + 1,
            _ => self.counter,
        }
    }
}

struct SafeState {
    counter: u8,
    tick: Option<f32>,
    open: Option<f32>,
}

/// Colour of every lock group in the level, keyed by the group's key index.
/// Assigned once from the level-start view: the palette cycles, and a group
/// skips any entry that is already taken or too close to the colour of the item
/// carrying its key, which is how the player tells lock and key apart.
#[derive(Clone, Default)]
pub struct LockColors(Vec<(u16, Color)>);

const LOCK_MIN_DISTANCE: f32 = 0.25;

impl LockColors {
    pub fn get(&self, key: u16) -> Color {
        self.0
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map_or(theme::LOCK_COLORS[0], |(_, color)| *color)
    }
}

fn linear_distance(a: Color, b: Color) -> f32 {
    let (a, b) = (a.to_linear(), b.to_linear());
    Vec3::new(a.red - b.red, a.green - b.green, a.blue - b.blue).length()
}

fn key_item_color(view: &BoardView, key: u16) -> Option<Color> {
    view.bottles
        .iter()
        .flat_map(|bottle| bottle.items.iter())
        .find(|item| item.key == key)
        .map(|item| theme::item_color(item.color))
}

pub fn lock_colors(view: &BoardView) -> LockColors {
    let palette = theme::LOCK_COLORS;
    let mut assigned: Vec<(u16, Color)> = Vec::new();

    for (index, (key, _)) in view.lock_groups.iter().enumerate() {
        let item = key_item_color(view, *key);
        let first = index % palette.len();
        let color = (0..palette.len())
            .map(|offset| palette[(first + offset) % palette.len()])
            .find(|candidate| {
                let free = !assigned.iter().any(|(_, taken)| *taken == *candidate);
                let distinct = item
                    .is_none_or(|color| linear_distance(*candidate, color) >= LOCK_MIN_DISTANCE);
                free && distinct
            })
            .unwrap_or(palette[first]);
        assigned.push((*key, color));
    }

    LockColors(assigned)
}

#[derive(Component)]
pub struct Door {
    first: u8,
    sparks: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum DoorRole {
    Half(f32),
    Lock,
    Shackle,
}

#[derive(Component)]
pub struct DoorPart {
    first: u8,
    tint: Color,
    role: DoorRole,
}

/// The key badge leaves its item and crosses the board, so it lives in board
/// space rather than inside the bottle that carried it.
#[derive(Component)]
pub struct FlyingKey {
    first: u8,
    from: (u8, u8),
    target: Vec2,
}

#[allow(clippy::too_many_arguments)]
fn spawn_door(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    view: &BoardView,
    color: Color,
    first: u8,
    last: u8,
) {
    let Some(span) = span_rect(geometry, view, first..=last) else {
        return;
    };
    let bounds = span.inflate(theme::DECOR_MARGIN);
    let center = bounds.center();
    let half = bounds.half_size();
    let leaf = Vec2::new(half.x, bounds.height());
    let border = theme::DOOR_BORDER;
    let field = leaf - Vec2::splat(2.0 * border);

    let door = commands
        .spawn((
            Door { first, sparks: 0 },
            Transform::from_translation(center.extend(theme::Z_DOOR)),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();

    for side in [-1.0f32, 1.0] {
        let anchor = if side < 0.0 {
            Anchor::CENTER_LEFT
        } else {
            Anchor::CENTER_RIGHT
        };
        let inward = -side * leaf.x * 0.5;
        let leaf_entity = commands
            .spawn((
                DoorPart {
                    first,
                    tint: theme::WOOD_FRAME,
                    role: DoorRole::Half(side),
                },
                Sprite {
                    image: art.door_plate.outer.clone(),
                    color: theme::WOOD_FRAME,
                    custom_size: Some(leaf),
                    image_mode: sliced(theme::DOOR_RADIUS),
                    ..default()
                },
                anchor,
                Transform::from_xyz(side * half.x, 0.0, 0.0),
                ChildOf(door),
            ))
            .id();
        commands.spawn((
            Sprite {
                image: art.door_plate.inner.clone(),
                color: theme::WOOD_PLANK,
                custom_size: Some(field),
                image_mode: sliced(theme::DOOR_RADIUS - border),
                ..default()
            },
            Transform::from_xyz(inward, 0.0, 0.05),
            ChildOf(leaf_entity),
        ));
        commands.spawn((
            Sprite {
                image: art.door_grooves.clone(),
                color: theme::WOOD_SHADE.with_alpha(theme::DOOR_GROOVE_ALPHA),
                custom_size: Some(field),
                image_mode: tiled(),
                ..default()
            },
            Transform::from_xyz(inward, 0.0, 0.1),
            ChildOf(leaf_entity),
        ));
    }

    let lock = commands
        .spawn((
            DoorPart {
                first,
                tint: color,
                role: DoorRole::Lock,
            },
            Sprite {
                image: art.lock_body.clone(),
                color,
                custom_size: Some(Vec2::new(theme::LOCK_W, theme::LOCK_H)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, theme::Z_DOOR_LOCK - theme::Z_DOOR),
            ChildOf(door),
        ))
        .id();
    commands.spawn((
        DoorPart {
            first,
            tint: theme::METAL_LIGHT,
            role: DoorRole::Shackle,
        },
        Sprite {
            image: art.lock_shackle.clone(),
            color: theme::METAL_LIGHT,
            custom_size: Some(Vec2::new(theme::LOCK_SHACKLE_W, theme::LOCK_SHACKLE_H)),
            ..default()
        },
        Anchor::BOTTOM_CENTER,
        Transform::from_xyz(0.0, theme::LOCK_H * 0.5 - 2.0, -0.05),
        ChildOf(lock),
    ));

    let Some(from) = view.bottles.iter().find_map(|bottle| {
        bottle
            .items
            .iter()
            .position(|item| item.key != 0 && item.key == key_of(view, first))
            .map(|index| (bottle.id, index as u8))
    }) else {
        return;
    };

    commands.spawn((
        FlyingKey {
            first,
            from,
            target: center,
        },
        Sprite {
            image: art.key.clone(),
            color,
            custom_size: Some(Vec2::new(theme::KEY_W, theme::KEY_H)),
            ..default()
        },
        Transform::from_translation(center.extend(theme::Z_KEY)),
        Visibility::Hidden,
        ChildOf(board),
    ));
}

fn key_of(view: &BoardView, first: u8) -> u16 {
    view.lock_groups
        .iter()
        .find(|(_, range)| range.start == first)
        .map_or(0, |(key, _)| *key)
}

fn spawn_color_curtain(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    bottle: &BottleView,
    color: u8,
) {
    let bounds = geometry
        .bottle_rect(bottle.slot())
        .inflate(theme::DECOR_MARGIN);
    let half = bounds.half_size();
    let strips = theme::COLOR_CURTAIN_STRIPS;
    let width = bounds.width() / strips as f32;

    let curtain = commands
        .spawn((
            ColorCurtain {
                bottle: bottle.id,
                height: bounds.height(),
                top: bounds.max.y,
            },
            Transform::from_xyz(bounds.center().x, bounds.max.y, theme::Z_COLOR_CURTAIN),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();

    for strip in 0..strips {
        let outer = strip == 0 || strip + 1 == strips;
        commands.spawn((
            ColorCurtainPart {
                bottle: bottle.id,
                tint: theme::COLOR_CURTAIN,
                role: ColorCurtainRole::Strip(strip),
            },
            Sprite {
                image: match outer {
                    true => art.color_end.clone(),
                    false => art.color_cloth.clone(),
                },
                color: theme::COLOR_CURTAIN,
                custom_size: Some(Vec2::new(width, bounds.height())),
                flip_x: outer && strip > 0,
                ..default()
            },
            Anchor::TOP_CENTER,
            Transform::from_xyz(-half.x + (strip as f32 + 0.5) * width, 0.0, 0.0),
            ChildOf(curtain),
        ));
    }

    for strip in 0..strips {
        commands.spawn((
            ColorCurtainPart {
                bottle: bottle.id,
                tint: theme::COLOR_CURTAIN,
                role: ColorCurtainRole::Hem(strip),
            },
            Sprite {
                image: art.color_hem.clone(),
                color: theme::COLOR_CURTAIN,
                custom_size: Some(Vec2::new(width, theme::COLOR_CURTAIN_HEM)),
                ..default()
            },
            Anchor::TOP_CENTER,
            Transform::from_xyz(-half.x + (strip as f32 + 0.5) * width, 0.0, 0.0),
            ChildOf(curtain),
        ));
    }

    let badge = Vec2::new(theme::COLOR_CURTAIN_ICON_W, theme::COLOR_CURTAIN_ICON_H);
    let at = -bounds.height() * theme::COLOR_CURTAIN_ICON_Y;
    commands.spawn((
        ColorCurtainPart {
            bottle: bottle.id,
            tint: theme::COLOR_CURTAIN_SHADE,
            role: ColorCurtainRole::Badge,
        },
        Sprite {
            image: art.bottle_icon.clone(),
            color: theme::COLOR_CURTAIN_SHADE,
            custom_size: Some(badge * theme::COLOR_CURTAIN_RING),
            ..default()
        },
        Transform::from_xyz(
            0.0,
            at,
            theme::Z_COLOR_CURTAIN_ICON - theme::Z_COLOR_CURTAIN,
        ),
        ChildOf(curtain),
    ));
    commands.spawn((
        ColorCurtainPart {
            bottle: bottle.id,
            tint: theme::item_color(color),
            role: ColorCurtainRole::Badge,
        },
        Sprite {
            image: art.bottle_icon.clone(),
            color: theme::item_color(color),
            custom_size: Some(badge),
            ..default()
        },
        Transform::from_xyz(
            0.0,
            at,
            theme::Z_COLOR_CURTAIN_ICON - theme::Z_COLOR_CURTAIN + 0.05,
        ),
        ChildOf(curtain),
    ));
}

#[derive(Component)]
pub struct ColorCurtain {
    bottle: u8,
    height: f32,
    top: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorCurtainRole {
    Strip(usize),
    Hem(usize),
    Badge,
}

#[derive(Component)]
pub struct ColorCurtainPart {
    bottle: u8,
    tint: Color,
    role: ColorCurtainRole,
}

/// How far the door of `first` has swung, `0.0` until the key has landed.
fn door_open(effects: Option<(&MovePlan, f32)>, first: u8) -> Option<f32> {
    delayed(
        effects,
        |plan| plan.unlock_run.is_some_and(|range| range.start == first),
        theme::LOCK_DELAY,
        theme::DOOR_OPEN,
    )
}

const LOCK_POP: f32 = 0.3;

pub fn sync_doors(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut doors: Query<(&Door, &mut Visibility)>,
    mut parts: Query<(&DoorPart, &mut Sprite, &mut Transform)>,
) {
    let effects = flow.effects();
    let mut states: Vec<(u8, f32)> = Vec::new();

    for (door, mut visibility) in &mut doors {
        let open = door_open(effects, door.first);
        let locked = view.get(door.first).locked;
        *visibility = shown(locked || open.is_some_and(|open| open < 1.0));
        states.push((door.first, open.unwrap_or(0.0)));
    }

    for (part, mut sprite, mut transform) in &mut parts {
        let Some((_, open)) = states.iter().copied().find(|(id, _)| *id == part.first) else {
            continue;
        };
        let pop = (open / LOCK_POP).clamp(0.0, 1.0);
        let fall = ((open - LOCK_POP) / (1.0 - LOCK_POP)).clamp(0.0, 1.0);

        match part.role {
            DoorRole::Half(side) => {
                let swing = ease(fall);
                transform.scale.x = 1.0 - theme::DOOR_SHUT * swing;
                transform.rotation = Quat::from_rotation_z(-side * theme::DOOR_SWING * swing);
                sprite.color = part.tint;
            }
            DoorRole::Lock => {
                transform.translation.y = -theme::LOCK_DROP * fall * fall;
                transform.rotation = Quat::from_rotation_z(theme::LOCK_TIP * fall);
                sprite.color = part
                    .tint
                    .with_alpha(1.0 - ((fall - 0.5) / 0.5).clamp(0.0, 1.0));
            }
            DoorRole::Shackle => {
                transform.translation.y = theme::LOCK_H * 0.5 - 2.0 + theme::LOCK_SHACKLE_POP * pop;
                sprite.color = part.tint;
            }
        }
    }
}

fn quadratic(a: Vec2, control: Vec2, b: Vec2, u: f32) -> Vec2 {
    let inv = 1.0 - u;
    a * inv * inv + control * 2.0 * inv * u + b * u * u
}

/// Board-space context for the key flight, bundled to keep the system short.
#[derive(SystemParam)]
pub struct KeyStage<'w> {
    art: Res<'w, Art>,
    view: Res<'w, BoardView>,
    geometry: Res<'w, BoardGeometry>,
    rng: ResMut<'w, DecorRng>,
}

pub fn sync_keys(
    mut commands: Commands,
    flow: Res<Flow>,
    mut stage: KeyStage,
    mut doors: Query<&mut Door>,
    mut keys: Query<(&FlyingKey, &Sprite, &mut Transform, &mut Visibility)>,
) {
    let effects = flow.effects();

    for (key, sprite, mut transform, mut visibility) in &mut keys {
        let flight = delayed(
            effects,
            |plan| {
                plan.key_used == Some(key.from)
                    && plan
                        .unlock_run
                        .is_some_and(|range| range.start == key.first)
            },
            0.0,
            theme::KEY_FLIGHT,
        );
        let Some(progress) = flight.filter(|progress| *progress < 1.0) else {
            *visibility = Visibility::Hidden;
            if let Some(mut door) = doors.iter_mut().find(|door| door.first == key.first)
                && flight.is_none()
            {
                door.sparks = 0;
            }
            continue;
        };

        let bottle = stage.view.get(key.from.0);
        let rect = stage.geometry.bottle_rect(bottle.slot());
        let from = stage.geometry.item_rect(rect, key.from.1).center();
        let eased = ease(progress);
        let control = from.midpoint(key.target) + Vec2::Y * theme::KEY_ARC;
        let at = quadratic(from, control, key.target, eased);

        *visibility = Visibility::Inherited;
        transform.translation = at.extend(theme::Z_KEY);
        transform.rotation = Quat::from_rotation_z(theme::KEY_SPIN * (1.0 - eased));
        transform.scale = Vec3::splat(theme::KEY_BADGE.lerp(1.0, eased));

        let Some(mut door) = doors.iter_mut().find(|door| door.first == key.first) else {
            continue;
        };
        let wanted = (progress * theme::KEY_SPARKS as f32).floor() as usize;
        while door.sparks < wanted {
            door.sparks += 1;
            spark(
                &mut commands,
                &stage.art,
                &mut stage.rng.0,
                at,
                sprite.color,
            );
        }
    }
}

fn spark(commands: &mut Commands, art: &Art, rng: &mut Pcg32, at: Vec2, color: Color) {
    commands.spawn((
        Particle {
            velocity: Vec2::new(
                rng.range(-1.0, 1.0) * theme::KEY_SPARK_SPEED,
                rng.range(-0.2, 1.0) * theme::KEY_SPARK_SPEED,
            ),
            gravity: theme::KEY_SPARK_GRAVITY,
            spin: rng.range(-2.0, 2.0),
            life: theme::KEY_SPARK_LIFE,
            max_life: theme::KEY_SPARK_LIFE,
        },
        Sprite {
            image: art.star.clone(),
            color: color.mix(&theme::STAR, 0.5),
            custom_size: Some(Vec2::splat(theme::KEY_SPARK_SIZE)),
            ..default()
        },
        Transform::from_translation(at.extend(theme::Z_KEY)),
    ));
}

/// How much of one strip is still hanging. `COLOR_CURTAIN_RIPPLE` staggers the
/// strips, so the hem riding each strip's bottom edge has to read the same
/// number.
fn drawn_height(progress: f32, strip: usize, height: f32) -> f32 {
    let u = (strip as f32 + 0.5) / theme::COLOR_CURTAIN_STRIPS as f32;
    let ripple = (u * 2.0 * TAU).sin() * 0.5 + 0.5;
    let ratio = theme::COLOR_CURTAIN_RIPPLE;
    let drawn = (ease(progress) * (1.0 + ratio) - ratio * ripple).clamp(0.0, 1.0);
    height * (1.0 - drawn)
}

pub fn sync_color_curtains(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut curtains: Query<(&ColorCurtain, &mut Transform, &mut Visibility)>,
    mut parts: Query<(&ColorCurtainPart, &mut Sprite, &mut Transform), Without<ColorCurtain>>,
) {
    let effects = flow.effects();
    let mut states: Vec<(u8, f32, f32)> = Vec::new();

    for (curtain, mut transform, mut visibility) in &mut curtains {
        let lift = phase(
            effects,
            |plan| plan.color_curtains_lifted.contains(&curtain.bottle),
            theme::COLOR_CURTAIN_LIFT,
        );
        let covered = view.get(curtain.bottle).color_curtain.is_some();
        let progress = lift.unwrap_or(0.0);

        *visibility = shown(covered || lift.is_some_and(|lift| lift < 1.0));
        transform.translation.y = curtain.top + theme::COLOR_CURTAIN_DRIFT * ease(progress);
        states.push((curtain.bottle, progress, curtain.height));
    }

    for (part, mut sprite, mut transform) in &mut parts {
        let Some((_, progress, height)) =
            states.iter().copied().find(|(id, ..)| *id == part.bottle)
        else {
            continue;
        };
        let alpha = 1.0 - (progress / 0.85).clamp(0.0, 1.0);
        sprite.color = part.tint.with_alpha(alpha);

        match part.role {
            ColorCurtainRole::Strip(strip) => {
                let size = sprite.custom_size.unwrap_or_default();
                sprite.custom_size = Some(Vec2::new(size.x, drawn_height(progress, strip, height)));
            }
            ColorCurtainRole::Hem(strip) => {
                transform.translation.y =
                    theme::COLOR_CURTAIN_HEM_LIP - drawn_height(progress, strip, height);
            }
            ColorCurtainRole::Badge => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::range::RangeInclusive;

    use bevy::prelude::Color;

    use super::{LOCK_MIN_DISTANCE, key_item_color, linear_distance, lock_colors, safe_groups};
    use crate::session::Session;
    use crate::solver::reachable;
    use crate::theme;
    use crate::view::{BoardView, BottleView, ItemView};

    const VAULT_RUN: usize = 7;
    const KEY_AND_DOOR: usize = 8;
    const EVERYTHING: usize = 10;

    fn item(color: u8, key: u16) -> ItemView {
        ItemView {
            color,
            hidden: false,
            locked: false,
            key,
        }
    }

    fn bottle(id: u8, items: Vec<ItemView>) -> BottleView {
        BottleView {
            id,
            lines: (0..1).into(),
            repr_column: id * 2,
            capacity: 4,
            items,
            finalized: false,
            immovable: false,
            pluggable: false,
            plugged: false,
            frozen: false,
            behind_curtain: false,
            safe_counter: 0,
            locked: false,
            filter_color: None,
            color_curtain: None,
            can_move_from: true,
            interactable: true,
        }
    }

    fn assert_readable(view: &BoardView) {
        let colors = lock_colors(view);
        let mut seen: Vec<Color> = Vec::new();
        for (key, _) in &view.lock_groups {
            let lock = colors.get(*key);
            let carried = key_item_color(view, *key).expect("the key sits on an item");
            assert!(
                linear_distance(lock, carried) >= LOCK_MIN_DISTANCE,
                "lock {lock:?} is too close to its key item {carried:?}"
            );
            assert!(!seen.contains(&lock), "two groups share {lock:?}");
            seen.push(lock);
        }
        assert_eq!(seen.len(), view.lock_groups.len());
    }

    /// Palette entry 0 is almost exactly item colour 7, so a group keyed by that
    /// colour has to skip it.
    #[test]
    fn a_lock_never_takes_the_colour_of_its_own_key() {
        let view = BoardView {
            bottles: vec![bottle(1, vec![item(7, 11)]), bottle(2, Vec::new())],
            frozen_ranges: Vec::new(),
            curtain_ranges: Vec::new(),
            lock_groups: vec![(11, RangeInclusive::from(2..=2))],
            solved: false,
            stuck: false,
            pours: None,
        };

        assert!(linear_distance(theme::LOCK_COLORS[0], theme::item_color(7)) < LOCK_MIN_DISTANCE);
        assert_ne!(lock_colors(&view).get(11), theme::LOCK_COLORS[0]);
        assert_readable(&view);
    }

    #[test]
    fn every_shipped_lock_group_is_readable() {
        for level in [KEY_AND_DOOR, EVERYTHING] {
            let session = Session::new(level).expect("level exists");
            let view = session.view();
            assert!(!view.lock_groups.is_empty());
            assert_readable(&view);
        }
    }

    #[test]
    fn a_safe_group_is_one_wide_door_over_equal_neighbours() {
        let session = Session::new(VAULT_RUN).expect("level exists");
        let groups = safe_groups(&session.view());

        assert_eq!(
            groups,
            vec![vec![6, 7], vec![8], vec![9], vec![12], vec![13]]
        );
    }

    /// The acceptance criterion for M5: a group that exists at level start
    /// either survives unchanged or disappears when its counter runs out. No
    /// state reachable from the start may produce a grouping that is not a
    /// subset of the original one.
    #[test]
    fn safe_groups_never_re_form() {
        let session = Session::new(VAULT_RUN).expect("level exists");
        let start = safe_groups(&session.view());
        assert!(start.len() > 1);

        let mut opened = 0;
        let mut states = 0;
        reachable(VAULT_RUN, 5_000, |view| {
            let groups = safe_groups(view);
            for group in &groups {
                assert!(
                    start.contains(group),
                    "grouping {groups:?} is not part of {start:?}"
                );
            }
            opened = opened.max(start.len() - groups.len());
            states += 1;
        });

        assert!(states > 1_000);
        assert_eq!(opened, start.len());
    }
}
