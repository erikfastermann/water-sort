use std::f32::consts::{PI, TAU};

use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2d};

use crate::anim::Flow;
use crate::art::{Art, crop, sliced};
use crate::fx::Particle;
use crate::geometry::{BoardGeometry, COL_PITCH, CURTAIN_H};
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CurtainRole {
    Sheet(u8),
    Trim,
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
}

fn bottle_rect(geometry: &BoardGeometry, view: &BoardView, id: u8) -> Rect {
    let bottle = view.get(id);
    geometry.bottle_rect(bottle.lines, bottle.repr_column)
}

fn span_rect(
    geometry: &BoardGeometry,
    view: &BoardView,
    ids: impl Iterator<Item = u8>,
) -> Option<Rect> {
    ids.map(|id| bottle_rect(geometry, view, id))
        .reduce(|span, rect| span.union(rect))
}

fn bar(tint: Color, size: Vec2, at: Vec2, z: f32) -> impl Bundle {
    (
        Sprite {
            color: tint,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(at.extend(z)),
    )
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
    let bounds = span.inflate(theme::ICE_MARGIN);
    let center = bounds.center();
    let half = bounds.half_size();

    let group = commands
        .spawn((
            IceGroup {
                first,
                bounds,
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
                tint: theme::ICE_BASE,
            },
            Sprite {
                image: art.ice_base.clone(),
                color: theme::ICE_BASE,
                custom_size: Some(Vec2::new(COL_PITCH, theme::ICE_BASE_H)),
                ..default()
            },
            Transform::from_xyz(column, span.min.y - center.y, theme::Z_ICE_BASE),
            ChildOf(group),
        ));
        commands.spawn((
            IcePart {
                first,
                tint: theme::ICE_FILL,
            },
            Sprite {
                image: art.ice_crown.clone(),
                color: theme::ICE_FILL,
                custom_size: Some(Vec2::new(COL_PITCH, theme::ICE_CROWN_H)),
                ..default()
            },
            Anchor::TOP_CENTER,
            Transform::from_xyz(column, half.y, theme::Z_ICE_FROST),
            ChildOf(group),
        ));
    }

    let frame = theme::ICE_FRAME_W;
    let edge = theme::ICE_EDGE.with_alpha(theme::ICE_FRAME_ALPHA);
    for side in [1.0, -1.0] {
        commands.spawn((
            IcePart { first, tint: edge },
            bar(
                edge,
                Vec2::new(frame, bounds.height()),
                Vec2::new(side * (half.x - frame * 0.5), 0.0),
                theme::Z_ICE_FRAME,
            ),
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
    let bounds = span.inflate(theme::CURTAIN_MARGIN);
    let center = bounds.center();
    let half = bounds.half_size();
    let left = -half.x;

    let curtain = commands
        .spawn((
            Curtain { first, last, left },
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
                image: art.curtain_cloth.clone(),
                color: theme::CURTAIN_CLOTH,
                custom_size: Some(Vec2::new(COL_PITCH, CURTAIN_H)),
                ..default()
            },
            Anchor::CENTER_LEFT,
            Transform::from_xyz(left + f32::from(column) * COL_PITCH, 0.0, theme::Z_CURTAIN),
            ChildOf(curtain),
        ));
    }

    for side in [1.0, -1.0] {
        commands.spawn((
            CurtainPart {
                first,
                role: CurtainRole::Trim,
            },
            Sprite {
                color: theme::CURTAIN_TRIM,
                custom_size: Some(Vec2::new(COL_PITCH, theme::CURTAIN_TRIM_H)),
                ..default()
            },
            Anchor::CENTER_LEFT,
            Transform::from_xyz(
                left,
                side * (half.y - theme::CURTAIN_TRIM_H * 0.5),
                theme::Z_CURTAIN_TRIM,
            ),
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
                custom_size: Some(Vec2::new(theme::CURTAIN_ROLL_W, CURTAIN_H)),
                ..default()
            },
            Transform::from_xyz(left, 0.0, theme::Z_CURTAIN_ROLL),
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
            Transform::from_xyz(0.0, side * half.y, 0.1),
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
    let bounds = span.inflate(theme::SAFE_MARGIN);
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
    effects
        .filter(|(plan, _)| active(plan))
        .map(|(_, clock)| ((clock - theme::RANGE_DELAY) / duration).clamp(0.0, 1.0))
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
    let states: Vec<(u8, f32, f32)> = curtains
        .iter()
        .map(|curtain| {
            (
                curtain.first,
                covered(&view, &flow, curtain.first, curtain.last),
                curtain.left,
            )
        })
        .collect();

    for (part, mut sprite, mut transform, mut visibility) in &mut parts {
        let Some((_, width, left)) = states.iter().copied().find(|(id, ..)| *id == part.first)
        else {
            continue;
        };

        match part.role {
            CurtainRole::Sheet(column) => {
                let fraction = (width - f32::from(column)).clamp(0.0, 1.0);
                *visibility = shown(fraction > 0.0);
                sprite.custom_size = Some(Vec2::new(fraction * COL_PITCH, CURTAIN_H));
                sprite.rect = Some(crop(Vec2::new(COL_PITCH, CURTAIN_H), fraction));

                let wave = (elapsed * theme::CURTAIN_WAVE_HZ * TAU + f32::from(column)).sin();
                transform.scale.y = 1.0 + theme::CURTAIN_WAVE * wave * (width.fract() * PI).sin();
            }
            CurtainRole::Trim => {
                *visibility = shown(width > 0.0);
                sprite.custom_size = Some(Vec2::new(width * COL_PITCH, theme::CURTAIN_TRIM_H));
            }
            CurtainRole::Roll => {
                *visibility = shown(width > 0.0);
                transform.translation.x = left + width * COL_PITCH;
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

#[cfg(test)]
mod tests {
    use super::safe_groups;
    use crate::session::Session;
    use crate::solver::reachable;

    const VAULT_RUN: usize = 7;

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
