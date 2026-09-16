use std::f32::consts::TAU;

use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::anim::Flow;
use crate::art::{Art, ITEM_W};
use crate::input::Selection;
use crate::theme;
use crate::view::{BoardView, BottleView, MovePlan};

use super::Fluids;
use super::bottle::{fill_level, top_of};
use super::decor::LockColors;

#[derive(Component)]
pub struct Question {
    bottle: u8,
    index: u8,
}

#[derive(Component)]
pub struct MetalBand {
    bottle: u8,
    index: u8,
}

/// Hinged at the item's upper right corner, with the lid sprite hanging back
/// over the item, so a rotation here swings the cover open.
#[derive(Component)]
pub struct MetalLid {
    bottle: u8,
    index: u8,
    open: f32,
}

#[derive(Component)]
pub struct KeyBadge {
    bottle: u8,
    index: u8,
    key: u16,
}

/// Hinge at the neck; the cord and the tag hang below it, so rotating this node
/// swings the whole tag.
#[derive(Component)]
pub struct FilterTag {
    bottle: u8,
}

#[derive(Component)]
pub struct PlugCollar {
    bottle: u8,
}

#[derive(Component)]
pub struct Plug {
    bottle: u8,
    seat: Vec2,
    hang: Vec2,
}

/// How black an item currently is: `1.0` while hidden, falling to `0.0` as it
/// is revealed. Also drives the question glyph, so both stay in step.
pub(super) fn hidden_level(
    bottle: &BottleView,
    index: u8,
    effects: Option<(&MovePlan, f32)>,
) -> f32 {
    if bottle
        .items
        .get(usize::from(index))
        .is_some_and(|item| item.hidden)
    {
        return 1.0;
    }
    match effects {
        Some((plan, clock)) if plan.reveal_item == Some((bottle.id, index)) => {
            1.0 - (clock / theme::REVEAL_ITEM).clamp(0.0, 1.0)
        }
        _ => 0.0,
    }
}

/// The lock is consumed by the pour that empties the item out, so the metal has
/// to keep rendering on the way out even though the resting view has dropped it.
fn locked(bottle: &BottleView, index: u8, effects: Option<(&MovePlan, f32)>) -> bool {
    bottle
        .items
        .get(usize::from(index))
        .is_some_and(|item| item.locked)
        || effects.is_some_and(|(plan, _)| plan.unlock_item == Some((bottle.id, index)))
}

pub fn spawn_item(
    commands: &mut Commands,
    slot: Entity,
    art: &Art,
    colors: &LockColors,
    view: &BottleView,
    index: u8,
) {
    let Some(item) = view.items.get(usize::from(index)) else {
        return;
    };

    if item.key != 0 {
        let color = colors.get(item.key);
        commands.spawn((
            KeyBadge {
                bottle: view.id,
                index,
                key: item.key,
            },
            Sprite {
                image: art.key.clone(),
                color,
                custom_size: Some(Vec2::new(theme::KEY_W, theme::KEY_H) * theme::KEY_BADGE),
                ..default()
            },
            Transform::from_xyz(0.0, theme::ITEM_H * 0.5, theme::Z_ITEM_KEY),
            ChildOf(slot),
        ));
    }

    if item.hidden {
        commands.spawn((
            Question {
                bottle: view.id,
                index,
            },
            Sprite {
                image: art.question.clone(),
                color: theme::ITEM_QUESTION,
                custom_size: Some(Vec2::new(theme::QUESTION_W, theme::QUESTION_H)),
                ..default()
            },
            Transform::from_xyz(0.0, theme::ITEM_H * 0.5, theme::Z_ITEM_QUESTION),
            ChildOf(slot),
        ));
    }

    if item.locked {
        commands.spawn((
            MetalBand {
                bottle: view.id,
                index,
            },
            Sprite {
                image: art.metal_band.clone(),
                color: theme::METAL_BAND,
                custom_size: Some(Vec2::new(ITEM_W, theme::ITEM_H)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, 0.0, theme::Z_ITEM_BAND),
            ChildOf(slot),
        ));

        let hinge = commands
            .spawn((
                MetalLid {
                    bottle: view.id,
                    index,
                    open: 0.0,
                },
                Transform::from_xyz(theme::LID_W * 0.5, theme::ITEM_H, theme::Z_ITEM_LID),
                Visibility::default(),
                ChildOf(slot),
            ))
            .id();
        commands.spawn((
            Sprite {
                image: art.metal_lid.clone(),
                color: theme::METAL_LIGHT,
                custom_size: Some(Vec2::new(theme::LID_W, theme::LID_H)),
                ..default()
            },
            Transform::from_xyz(-theme::LID_W * 0.5, -theme::LID_H * 0.5, 0.0),
            ChildOf(hinge),
        ));
    }
}

pub fn spawn_bottle(
    commands: &mut Commands,
    holder: Entity,
    art: &Art,
    view: &BottleView,
    base_y: f32,
    mouth_y: f32,
) {
    if view.immovable {
        commands.spawn((
            Sprite {
                image: art.rock_base.clone(),
                color: theme::ROCK_LIGHT,
                custom_size: Some(Vec2::new(theme::ROCK_W, theme::ROCK_H)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, base_y - theme::ROCK_DROP, theme::Z_ROCKS),
            ChildOf(holder),
        ));
    }

    if let Some(color) = view.filter_color {
        let hinge = commands
            .spawn((
                FilterTag { bottle: view.id },
                Transform::from_xyz(
                    theme::TAG_HANG.x,
                    mouth_y + theme::TAG_HANG.y,
                    theme::Z_TAG_CORD,
                ),
                Visibility::default(),
                ChildOf(holder),
            ))
            .id();
        commands.spawn((
            Sprite {
                color: theme::ROPE,
                custom_size: Some(Vec2::new(theme::TAG_CORD_W, theme::TAG_CORD_H)),
                ..default()
            },
            Anchor::TOP_CENTER,
            Transform::from_xyz(0.0, 0.0, 0.0),
            ChildOf(hinge),
        ));
        commands.spawn((
            Sprite {
                image: art.tag.clone(),
                color: theme::item_color(color),
                custom_size: Some(Vec2::new(theme::TAG_W, theme::TAG_H)),
                ..default()
            },
            Anchor::TOP_CENTER,
            Transform::from_xyz(
                0.0,
                -theme::TAG_CORD_H + 3.0,
                theme::Z_TAG - theme::Z_TAG_CORD,
            ),
            ChildOf(hinge),
        ));
    }

    if view.pluggable {
        commands.spawn((
            PlugCollar { bottle: view.id },
            Sprite {
                image: art.rope.clone(),
                color: theme::ROPE,
                custom_size: Some(Vec2::new(theme::ROPE_W, theme::ROPE_H)),
                ..default()
            },
            Transform::from_xyz(0.0, mouth_y - theme::ROPE_DROP, theme::Z_ROPE),
            ChildOf(holder),
        ));

        let seat = Vec2::new(0.0, mouth_y) + theme::PLUG_SEAT;
        commands.spawn((
            Plug {
                bottle: view.id,
                seat,
                hang: Vec2::new(0.0, mouth_y) + theme::PLUG_HANG,
            },
            Sprite {
                image: art.plug.clone(),
                color: theme::PLUG,
                custom_size: Some(Vec2::new(theme::PLUG_W, theme::PLUG_H)),
                ..default()
            },
            Transform::from_translation(seat.extend(theme::Z_PLUG)),
            ChildOf(holder),
        ));
    }
}

pub fn sync_questions(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut glyphs: Query<(&Question, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let pour = flow.pour();
    let effects = flow.effects();
    for (glyph, mut sprite, mut transform, mut visibility) in &mut glyphs {
        let bottle = view.get(glyph.bottle);
        let level = hidden_level(bottle, glyph.index, effects);
        if level <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }

        let fill = (fill_level(bottle, pour.as_ref()) - f32::from(glyph.index)).clamp(0.0, 1.0);
        *visibility = Visibility::Inherited;
        transform.translation.y = fill * theme::ITEM_H * 0.5;
        transform.scale = Vec3::splat(level.max(f32::EPSILON));
        sprite.color = theme::ITEM_QUESTION.with_alpha(level);
    }
}

pub fn sync_bands(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut bands: Query<(&MetalBand, &mut Sprite, &mut Visibility)>,
) {
    let pour = flow.pour();
    let effects = flow.effects();
    for (band, mut sprite, mut visibility) in &mut bands {
        let bottle = view.get(band.bottle);
        let fill = (fill_level(bottle, pour.as_ref()) - f32::from(band.index)).clamp(0.0, 1.0);

        if fill <= 0.0 || !locked(bottle, band.index, effects) {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;
        sprite.custom_size = Some(Vec2::new(ITEM_W, fill * theme::ITEM_H));
    }
}

pub fn sync_lids(
    time: Res<Time>,
    view: Res<BoardView>,
    flow: Res<Flow>,
    selection: Res<Selection>,
    mut lids: Query<(&mut MetalLid, &mut Transform, &mut Visibility)>,
) {
    let dt = time.delta_secs().min(1.0 / 30.0);
    let pour = flow.pour();
    let effects = flow.effects();
    for (mut lid, mut transform, mut visibility) in &mut lids {
        let bottle = view.get(lid.bottle);
        let level = fill_level(bottle, pour.as_ref());
        let fill = (level - f32::from(lid.index)).clamp(0.0, 1.0);
        let shown = fill > 0.0 && locked(bottle, lid.index, effects);

        let lifting = selection.get() == Some(lid.bottle)
            || pour
                .as_ref()
                .is_some_and(|state| state.plan.from == lid.bottle);
        let target = if lifting && top_of(level) == Some(lid.index) {
            1.0
        } else {
            0.0
        };
        lid.open = lid
            .open
            .lerp(target, (dt * theme::LID_RATE).clamp(0.0, 1.0));

        *visibility = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        transform.translation.y = fill * theme::ITEM_H;
        transform.rotation = Quat::from_rotation_z(theme::LID_ANGLE * lid.open);
    }
}

/// A pluggable bottle that has been emptied loses its plug for good; the view
/// reports that as `!pluggable`, and the effects phase fades it out.
fn plug_alpha(bottle: &BottleView, effects: Option<(&MovePlan, f32)>) -> f32 {
    if bottle.finalized {
        return 0.0;
    }
    if bottle.pluggable {
        return 1.0;
    }
    match effects {
        Some((plan, clock)) if plan.unplug == Some(bottle.id) => {
            1.0 - (clock / theme::PLUG_TOGGLE).clamp(0.0, 1.0)
        }
        _ => 0.0,
    }
}

fn arc(from: Vec2, to: Vec2, u: f32) -> Vec2 {
    from.lerp(to, u) - Vec2::X * theme::PLUG_ARC * 4.0 * u * (1.0 - u)
}

pub fn sync_plugs(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut collars: Query<(&PlugCollar, &mut Sprite, &mut Visibility), Without<Plug>>,
    mut plugs: Query<(&Plug, &mut Sprite, &mut Transform, &mut Visibility), Without<PlugCollar>>,
) {
    let effects = flow.effects();

    for (collar, mut sprite, mut visibility) in &mut collars {
        let alpha = plug_alpha(view.get(collar.bottle), effects);
        *visibility = if alpha > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        sprite.color = theme::ROPE.with_alpha(alpha);
    }

    for (plug, mut sprite, mut transform, mut visibility) in &mut plugs {
        let bottle = view.get(plug.bottle);
        let alpha = plug_alpha(bottle, effects);
        *visibility = if alpha > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        sprite.color = theme::PLUG.with_alpha(alpha);

        let toggling = effects
            .filter(|(plan, _)| plan.plug_toggled.contains(&plug.bottle))
            .map(|(_, clock)| (clock / theme::PLUG_TOGGLE).clamp(0.0, 1.0));
        let progress = toggling.unwrap_or(1.0);
        let was = if toggling.is_some() {
            !bottle.plugged
        } else {
            bottle.plugged
        };

        let pose = |plugged: bool| {
            if plugged {
                (plug.seat, 0.0)
            } else {
                (plug.hang, theme::PLUG_HANG_ANGLE)
            }
        };
        let (from, from_angle) = pose(was);
        let (to, to_angle) = pose(bottle.plugged);
        let eased = EaseFunction::BackOut.sample_clamped(progress);

        transform.translation = arc(from, to, eased).extend(theme::Z_PLUG);
        transform.rotation = Quat::from_rotation_z(from_angle.lerp(to_angle, eased));
    }
}

/// The key leaves the item the moment its flight starts, but the resting view
/// has already dropped it, so the badge has to keep rendering until then.
fn carries_key(
    bottle: &BottleView,
    index: u8,
    key: u16,
    effects: Option<(&MovePlan, f32)>,
) -> bool {
    bottle
        .items
        .get(usize::from(index))
        .is_some_and(|item| item.key == key)
        || effects
            .is_some_and(|(plan, clock)| clock < 0.0 && plan.key_used == Some((bottle.id, index)))
}

pub fn sync_key_badges(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut badges: Query<(&KeyBadge, &mut Transform, &mut Visibility)>,
) {
    let pour = flow.pour();
    let effects = flow.effects();
    for (badge, mut transform, mut visibility) in &mut badges {
        let bottle = view.get(badge.bottle);
        let fill = (fill_level(bottle, pour.as_ref()) - f32::from(badge.index)).clamp(0.0, 1.0);
        let hidden = hidden_level(bottle, badge.index, effects);
        let shown =
            fill > 0.0 && hidden <= 0.0 && carries_key(bottle, badge.index, badge.key, effects);

        *visibility = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        transform.translation.y = fill * theme::ITEM_H * 0.5;
    }
}

/// The tag hangs off the neck, so it counter-rotates against the glass and
/// borrows the fluid spring for its swing.
pub fn sync_tags(
    time: Res<Time>,
    fluids: Res<Fluids>,
    mut tags: Query<(&FilterTag, &mut Transform)>,
) {
    let elapsed = time.elapsed_secs();
    for (tag, mut transform) in &mut tags {
        let fluid = fluids.get(tag.bottle);
        let sway = (elapsed * theme::TAG_SWAY_HZ * TAU + f32::from(tag.bottle)).sin();
        let swing =
            (fluid.tilt * theme::TAG_SWING).clamp(-theme::TAG_MAX_SWING, theme::TAG_MAX_SWING);
        transform.rotation = Quat::from_rotation_z(theme::TAG_SWAY * sway + swing - fluid.glass);
    }
}
