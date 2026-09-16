use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::anim::{Flow, PourState, pour_motion};
use crate::art::{ITEM_SURFACE_H, ITEM_W, glass_size};
use crate::geometry::{BoardGeometry, INTRO_ENTRY, check_capacity};
use crate::input::Selection;
use crate::theme;
use crate::view::{BoardView, BottleView, MAX_BOTTLES};

use super::Painter;
use super::decor::LockColors;
use super::feature;

/// Fixed home position of a bottle; never moves.
#[derive(Component)]
pub struct BottleSlot;

/// Carries the lift and pour offsets away from the home position.
#[derive(Component)]
pub struct BottleRoot {
    pub id: u8,
}

/// Sits at the mouth so a rotation on it pivots around the rim; [`BottleArt`]
/// undoes the offset for the sprites below.
#[derive(Component)]
pub struct BottleTilt {
    pub id: u8,
}

#[derive(Component)]
pub struct BottleArt;

/// The only pickable entity of a bottle. It hangs off [`BottleSlot`], so the
/// hit rect stays at the home position while the bottle lifts or pours.
#[derive(Component)]
pub struct HitTarget {
    pub id: u8,
}

#[derive(Component)]
pub struct Halo {
    pub id: u8,
    alpha: f32,
}

#[derive(Component)]
pub struct Cork {
    pub id: u8,
    home_y: f32,
}

#[derive(Component)]
pub struct ItemSlot {
    pub bottle: u8,
    pub index: u8,
}

#[derive(Component)]
pub struct ItemSurface {
    pub bottle: u8,
    pub index: u8,
}

#[derive(Component, Default)]
pub struct Lift {
    value: f32,
    velocity: f32,
}

#[derive(Component)]
pub struct IntroSlide {
    delay: f32,
    from: Vec2,
    landed: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Fluid {
    pub tilt: f32,
    pub velocity: f32,
    /// Current rotation of the glass, so a surface can counter-rotate to stay
    /// level in world space.
    pub glass: f32,
    last_x: f32,
    last_velocity_x: f32,
    tracked: bool,
}

/// The spring lives in a resource rather than on the bottle entities because
/// the item surfaces that read it are two levels below the bottle root.
#[derive(Resource)]
pub struct Fluids([Fluid; MAX_BOTTLES]);

impl Default for Fluids {
    fn default() -> Self {
        Self([Fluid::default(); MAX_BOTTLES])
    }
}

impl Fluids {
    pub fn get(&self, id: u8) -> Fluid {
        self.0[usize::from(id)]
    }

    pub fn kick(&mut self, id: u8, impulse: f32) {
        self.0[usize::from(id)].velocity += impulse;
    }

    fn set_glass(&mut self, id: u8, angle: f32) {
        self.0[usize::from(id)].glass = angle;
    }

    fn integrate(&mut self, id: u8, x: f32, dt: f32) {
        let fluid = &mut self.0[usize::from(id)];
        let velocity_x = if fluid.tracked {
            (x - fluid.last_x) / dt
        } else {
            0.0
        };
        let acceleration = if fluid.tracked {
            (velocity_x - fluid.last_velocity_x) / dt
        } else {
            0.0
        };
        fluid.tracked = true;
        fluid.last_x = x;
        fluid.last_velocity_x = velocity_x;

        let drive = (-acceleration * theme::FLUID_DRIVE)
            .clamp(-theme::FLUID_DRIVE_MAX, theme::FLUID_DRIVE_MAX);
        fluid.velocity +=
            (drive - theme::FLUID_STIFFNESS * fluid.tilt - theme::FLUID_DAMPING * fluid.velocity)
                * dt;
        fluid.tilt =
            (fluid.tilt + fluid.velocity * dt).clamp(-theme::FLUID_MAX_TILT, theme::FLUID_MAX_TILT);
    }
}

pub fn spawn(
    commands: &mut Commands,
    board: Entity,
    painter: &mut Painter,
    geometry: &BoardGeometry,
    colors: &LockColors,
    view: &BottleView,
) {
    let art = &painter.art;
    check_capacity(view.capacity, view.line_span());

    let bounds = geometry.bottle_rect(view.slot());
    let center = bounds.center();
    let pivot = geometry.mouth(bounds) - center;
    let entry = if view.lines.start.is_multiple_of(2) {
        -INTRO_ENTRY
    } else {
        INTRO_ENTRY
    };

    let slot = commands
        .spawn((
            BottleSlot,
            Transform::from_xyz(center.x, center.y, 0.0),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();
    commands.spawn((
        HitTarget { id: view.id },
        Sprite {
            color: Color::NONE,
            custom_size: Some(bounds.size()),
            ..default()
        },
        Transform::default(),
        Pickable::default(),
        ChildOf(slot),
    ));

    let root = commands
        .spawn((
            BottleRoot { id: view.id },
            Lift::default(),
            IntroSlide {
                delay: f32::from(view.repr_column) * theme::INTRO_STAGGER,
                from: Vec2::new(entry, 0.0),
                landed: false,
            },
            Transform::from_xyz(entry, 0.0, theme::Z_BOTTLE),
            Visibility::default(),
            ChildOf(slot),
        ))
        .id();
    commands.spawn((
        Halo {
            id: view.id,
            alpha: 0.0,
        },
        Sprite {
            image: art.glow.clone(),
            color: theme::SELECT_GLOW.with_alpha(0.0),
            custom_size: Some(Vec2::new(
                theme::HALO_WIDTH,
                bounds.height() + theme::HALO_MARGIN,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_HALO),
        ChildOf(root),
    ));

    let tilt = commands
        .spawn((
            BottleTilt { id: view.id },
            Transform::from_translation(pivot.extend(0.0)),
            Visibility::default(),
            ChildOf(root),
        ))
        .id();
    let holder = commands
        .spawn((
            BottleArt,
            Transform::from_translation(-pivot.extend(0.0)),
            Visibility::default(),
            ChildOf(tilt),
        ))
        .id();

    let (glass_back, glass_front) = painter.glass.get(&mut painter.images, view.capacity);
    let glass = Some(glass_size(view.capacity));
    commands.spawn((
        Sprite {
            image: glass_back,
            color: theme::GLASS_INTERIOR,
            custom_size: glass,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_GLASS_BACK),
        ChildOf(holder),
    ));

    for index in 0..view.capacity {
        let item = geometry.item_rect(bounds, index);
        let image = if index == 0 {
            art.item_base.clone()
        } else {
            art.item_body.clone()
        };
        let slot = commands
            .spawn((
                ItemSlot {
                    bottle: view.id,
                    index,
                },
                Sprite {
                    image,
                    custom_size: Some(Vec2::new(ITEM_W, theme::ITEM_H)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(0.0, item.min.y - center.y, theme::Z_ITEM),
                ChildOf(holder),
            ))
            .id();
        commands.spawn((
            ItemSurface {
                bottle: view.id,
                index,
            },
            Sprite {
                image: art.item_surface.clone(),
                custom_size: Some(Vec2::new(ITEM_W, ITEM_SURFACE_H)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, theme::ITEM_H, theme::Z_ITEM_SURFACE),
            ChildOf(slot),
        ));
        feature::spawn_item(commands, slot, art, colors, view, index);
    }

    commands.spawn((
        Sprite {
            image: glass_front,
            color: theme::GLASS_RIM,
            custom_size: glass,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_GLASS_FRONT),
        ChildOf(holder),
    ));

    feature::spawn_bottle(
        commands,
        holder,
        art,
        view,
        bounds.min.y - center.y,
        pivot.y,
    );

    let cork_y = pivot.y + theme::CORK_H * 0.25;
    let cork = commands
        .spawn((
            Cork {
                id: view.id,
                home_y: cork_y,
            },
            Sprite {
                image: art.cork.clone(),
                color: theme::CORK_BODY,
                custom_size: Some(Vec2::new(theme::CORK_W, theme::CORK_H)),
                ..default()
            },
            Transform::from_xyz(0.0, cork_y, theme::Z_CORK),
            Visibility::Hidden,
            ChildOf(holder),
        ))
        .id();
    commands.spawn((
        Sprite {
            image: art.cork_cap.clone(),
            color: theme::CORK_TOP,
            custom_size: Some(Vec2::new(theme::CORK_W, theme::CORK_CAP_H)),
            ..default()
        },
        Transform::from_xyz(
            0.0,
            (theme::CORK_H - theme::CORK_CAP_H) * 0.5,
            theme::Z_ITEM_SURFACE,
        ),
        ChildOf(cork),
    ));
}

/// How many items the bottle currently shows. During a pour this runs ahead of
/// or behind the resting view, which already reflects the applied move.
pub(super) fn fill_level(view: &BottleView, pour: Option<&PourState>) -> f32 {
    match pour {
        Some(state) if state.plan.from == view.id => {
            f32::from(state.plan.from_height_before) - state.poured
        }
        Some(state) if state.plan.to == view.id => {
            f32::from(state.plan.to_height_before) + state.poured
        }
        _ => view.items.len() as f32,
    }
}

/// Index of the item carrying the liquid surface, for a fill level that may be
/// mid-drain.
pub(super) fn top_of(level: f32) -> Option<u8> {
    (level > 0.0).then(|| level.ceil() as u8 - 1)
}

fn item_color(view: &BottleView, index: u8, pour: Option<&PourState>) -> Option<u8> {
    view.items
        .get(usize::from(index))
        .map(|item| item.color)
        .or_else(|| {
            pour.filter(|state| state.plan.from == view.id)
                .map(|state| state.plan.color)
        })
}

pub fn apply_offsets(
    time: Res<Time>,
    flow: Res<Flow>,
    view: Res<BoardView>,
    geometry: Res<BoardGeometry>,
    selection: Res<Selection>,
    mut fluids: ResMut<Fluids>,
    mut roots: Query<(&BottleRoot, &mut Lift, &mut IntroSlide, &mut Transform)>,
) {
    let dt = time.delta_secs().min(1.0 / 30.0);
    let intro = flow.intro();
    let pour = flow.pour();
    let motion = pour
        .as_ref()
        .map(|state| pour_motion(state, &geometry, &view));

    for (root, mut lift, mut slide, mut transform) in &mut roots {
        let target = if selection.get() == Some(root.id) {
            theme::SELECT_RISE
        } else {
            0.0
        };
        lift.velocity += ((target - lift.value) * theme::LIFT_STIFFNESS
            - lift.velocity * theme::LIFT_DAMPING)
            * dt;
        lift.value += lift.velocity * dt;

        let progress = match intro {
            Some(clock) => ((clock - slide.delay) / theme::INTRO_SLIDE).clamp(0.0, 1.0),
            None => 1.0,
        };
        let slid = slide
            .from
            .lerp(Vec2::ZERO, EaseFunction::CubicOut.sample_clamped(progress));
        if progress >= 1.0 && !slide.landed {
            slide.landed = true;
            let sign = if root.id.is_multiple_of(2) { 1.0 } else { -1.0 };
            fluids.kick(root.id, sign * theme::FLUID_LAND_KICK);
        }

        let mut offset = slid + Vec2::Y * lift.value;
        let mut z = theme::Z_BOTTLE;
        if let (Some(state), Some(motion)) = (&pour, &motion)
            && state.plan.from == root.id
        {
            offset += motion.offset;
            z = theme::Z_POUR;
        }
        transform.translation = offset.extend(z);
    }
}

pub fn apply_tilts(
    flow: Res<Flow>,
    view: Res<BoardView>,
    geometry: Res<BoardGeometry>,
    mut fluids: ResMut<Fluids>,
    mut tilts: Query<(&BottleTilt, &mut Transform)>,
) {
    let pour = flow.pour();
    let motion = pour
        .as_ref()
        .map(|state| pour_motion(state, &geometry, &view));

    for (tilt, mut transform) in &mut tilts {
        let angle = match (&pour, &motion) {
            (Some(state), Some(motion)) if state.plan.from == tilt.id => motion.angle,
            _ => 0.0,
        };
        fluids.set_glass(tilt.id, angle);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

pub fn apply_fluid(
    time: Res<Time>,
    mut fluids: ResMut<Fluids>,
    roots: Query<(&BottleRoot, &Transform)>,
) {
    let dt = time.delta_secs().clamp(1.0 / 240.0, 1.0 / 30.0);
    for (root, transform) in &roots {
        fluids.integrate(root.id, transform.translation.x, dt);
    }
}

pub fn sync_items(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut slots: Query<(&ItemSlot, &mut Sprite, &mut Visibility)>,
) {
    let pour = flow.pour();
    let effects = flow.effects();
    for (slot, mut sprite, mut visibility) in &mut slots {
        let bottle = view.get(slot.bottle);
        let level = fill_level(bottle, pour.as_ref());
        let fill = (level - f32::from(slot.index)).clamp(0.0, 1.0);
        let color = item_color(bottle, slot.index, pour.as_ref());

        match (fill > 0.0).then_some(color).flatten() {
            Some(color) => {
                *visibility = Visibility::Inherited;
                sprite.custom_size = Some(Vec2::new(ITEM_W, fill * theme::ITEM_H));
                sprite.color = theme::item_color(color).mix(
                    &theme::ITEM_HIDDEN,
                    feature::hidden_level(bottle, slot.index, effects),
                );
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// The liquid surface only belongs on the topmost item; it stays a child of
/// that item so it is hidden and moved along with it.
pub fn sync_surfaces(
    view: Res<BoardView>,
    flow: Res<Flow>,
    fluids: Res<Fluids>,
    mut surfaces: Query<(&ItemSurface, &mut Sprite, &mut Visibility, &mut Transform)>,
) {
    let pour = flow.pour();
    let effects = flow.effects();
    for (surface, mut sprite, mut visibility, mut transform) in &mut surfaces {
        let bottle = view.get(surface.bottle);
        let level = fill_level(bottle, pour.as_ref());
        let fill = (level - f32::from(surface.index)).clamp(0.0, 1.0);
        let topmost = fill > 0.0 && top_of(level) == Some(surface.index);
        let color = item_color(bottle, surface.index, pour.as_ref());

        match topmost.then_some(color).flatten() {
            Some(color) => {
                *visibility = Visibility::Inherited;
                sprite.color = theme::item_color(color).mix(
                    &theme::ITEM_HIDDEN,
                    feature::hidden_level(bottle, surface.index, effects),
                );
                let top = fill * theme::ITEM_H;
                let overlap = ITEM_SURFACE_H.min(top);
                sprite.custom_size = Some(Vec2::new(ITEM_W, overlap));
                transform.translation.y = top - overlap;
                let fluid = fluids.get(surface.bottle);
                let level = (fluid.tilt - fluid.glass)
                    .clamp(-theme::SURFACE_MAX_TILT, theme::SURFACE_MAX_TILT);
                transform.rotation = Quat::from_rotation_z(level);
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

pub fn apply_corks(
    view: Res<BoardView>,
    flow: Res<Flow>,
    mut corks: Query<(&Cork, &mut Transform, &mut Visibility)>,
) {
    for (cork, mut transform, mut visibility) in &mut corks {
        let finalized = view.get(cork.id).finalized;
        let (shown, rise, scale) = match (finalized, flow.cork_progress(cork.id)) {
            (false, _) => (false, 0.0, 1.0),
            (true, None) => (true, 0.0, 1.0),
            (true, Some(elapsed)) if elapsed < theme::FINALIZE_SWIRL => (false, 0.0, 1.0),
            (true, Some(elapsed)) => {
                let dropped = EaseFunction::BackOut
                    .sample_clamped((elapsed - theme::FINALIZE_SWIRL) / theme::CORK_DROP);
                (
                    true,
                    theme::CORK_RISE * (1.0 - dropped),
                    theme::CORK_SQUASH.lerp(1.0, dropped),
                )
            }
        };

        *visibility = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        transform.translation.y = cork.home_y + rise;
        transform.scale = Vec3::new(1.0, scale, 1.0);
    }
}

pub fn apply_halos(
    time: Res<Time>,
    selection: Res<Selection>,
    mut halos: Query<(&mut Halo, &mut Sprite)>,
) {
    let dt = time.delta_secs().min(1.0 / 30.0);
    for (mut halo, mut sprite) in &mut halos {
        let target = if selection.get() == Some(halo.id) {
            1.0
        } else {
            0.0
        };
        halo.alpha = halo
            .alpha
            .lerp(target, (dt * theme::HALO_FADE).clamp(0.0, 1.0));
        sprite.color = theme::SELECT_GLOW.with_alpha(halo.alpha * theme::HALO_ALPHA);
    }
}
