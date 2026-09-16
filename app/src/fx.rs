use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::art::Art;
use crate::geometry::{Anchored, Band, PLAY_CENTER};
use crate::rng::Pcg32;
use crate::theme;

const STAR_SEED: u64 = 0x5EED_5741;
const STAR_SPREAD: Vec2 = Vec2::new(0.94, 0.96);

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(theme::BACKGROUND_BASE))
            .insert_resource(PopClock(theme::STAR_POP_INTERVAL))
            .insert_resource(StarRng(Pcg32::new(STAR_SEED, 1)))
            .add_systems(Startup, spawn_background)
            .add_systems(Update, (fit_viewport, animate_stars, drive_particles));
    }
}

#[derive(Resource)]
struct StarRng(Pcg32);

#[derive(Resource)]
struct PopClock(f32);

/// Sized and centred on the live camera area every frame, so a sprite covers
/// the viewport at any aspect ratio.
#[derive(Component)]
pub struct FitViewport;

/// Invisible full-viewport pick target. Clicks that miss every bottle land
/// here, which is what deselects and what cancels a running animation.
#[derive(Component)]
pub struct Backdrop;

/// Incidental debris. Never part of [`crate::anim::Flow`]; cancelling an
/// animation simply despawns whatever is still in flight.
#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub gravity: f32,
    pub spin: f32,
    pub life: f32,
    pub max_life: f32,
}

#[derive(Component)]
struct Star {
    unit: Vec2,
    size: f32,
    period: f32,
    phase: f32,
    pop: f32,
}

fn spawn_background(mut commands: Commands, art: Res<Art>, mut rng: ResMut<StarRng>) {
    commands.spawn((
        Backdrop,
        Sprite {
            color: Color::NONE,
            custom_size: Some(Vec2::new(theme::CANVAS_W, theme::CANVAS_H)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_BACKGROUND),
        FitViewport,
        Pickable::default(),
    ));

    commands.spawn((
        Sprite {
            image: art.glow.clone(),
            color: theme::BACKGROUND_GLOW.with_alpha(theme::GLOW_ALPHA),
            custom_size: Some(Vec2::splat(theme::GLOW_SIZE)),
            ..default()
        },
        Transform::from_xyz(PLAY_CENTER.x, PLAY_CENTER.y, theme::Z_GLOW),
        Anchored::new(Band::Play, Vec2::ZERO),
        Pickable::IGNORE,
    ));

    commands.spawn((
        Sprite {
            image: art.vignette.clone(),
            color: theme::BACKGROUND_EDGE.with_alpha(theme::VIGNETTE_ALPHA),
            custom_size: Some(Vec2::new(theme::CANVAS_W, theme::CANVAS_H)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_VIGNETTE),
        FitViewport,
        Pickable::IGNORE,
    ));

    for _ in 0..theme::STAR_COUNT {
        let unit = Vec2::new(
            rng.0.range(-0.5, 0.5) * STAR_SPREAD.x,
            rng.0.range(-0.5, 0.5) * STAR_SPREAD.y,
        );
        let size = rng.0.range(theme::STAR_SIZE_MIN, theme::STAR_SIZE_MAX);
        commands.spawn((
            Sprite {
                image: art.star.clone(),
                color: theme::STAR,
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, theme::Z_STARS),
            Star {
                unit,
                size,
                period: rng.0.range(theme::STAR_PERIOD_MIN, theme::STAR_PERIOD_MAX),
                phase: rng.0.range(0.0, TAU),
                pop: 0.0,
            },
            Pickable::IGNORE,
        ));
    }
}

fn view_area(projection: &Projection) -> Option<Rect> {
    match projection {
        Projection::Orthographic(orthographic) => Some(orthographic.area),
        _ => None,
    }
}

fn fit_viewport(
    camera: Single<&Projection, With<Camera2d>>,
    mut sprites: Query<(&mut Sprite, &mut Transform), With<FitViewport>>,
) {
    let Some(area) = view_area(&camera) else {
        return;
    };
    for (mut sprite, mut transform) in &mut sprites {
        sprite.custom_size = Some(area.size());
        transform.translation.x = area.center().x;
        transform.translation.y = area.center().y;
    }
}

fn animate_stars(
    time: Res<Time>,
    camera: Single<&Projection, With<Camera2d>>,
    mut clock: ResMut<PopClock>,
    mut rng: ResMut<StarRng>,
    mut stars: Query<(&mut Star, &mut Sprite, &mut Transform)>,
) {
    let Some(area) = view_area(&camera) else {
        return;
    };

    let delta = time.delta_secs();
    let elapsed = time.elapsed_secs();

    clock.0 -= delta;
    let popped = if clock.0 <= 0.0 {
        clock.0 = theme::STAR_POP_INTERVAL;
        Some(rng.0.below(stars.iter().len().max(1) as u32) as usize)
    } else {
        None
    };

    for (index, (mut star, mut sprite, mut transform)) in stars.iter_mut().enumerate() {
        if popped == Some(index) {
            star.pop = theme::STAR_POP_TIME;
        }
        star.pop = (star.pop - delta).max(0.0);

        let wave = (elapsed / star.period * TAU + star.phase).sin() * 0.5 + 0.5;
        let twinkle = wave * wave * (3.0 - 2.0 * wave);
        let pop = (star.pop / theme::STAR_POP_TIME * std::f32::consts::PI).sin();

        let alpha = (0.15 + 0.65 * twinkle + 0.35 * pop).min(1.0);
        let scale = 0.5 + 0.5 * alpha + (theme::STAR_POP_SCALE - 1.0) * pop;

        sprite.color = theme::STAR.with_alpha(alpha);
        sprite.custom_size = Some(Vec2::splat(star.size * scale));
        transform.translation.x = area.center().x + star.unit.x * area.width();
        transform.translation.y = area.center().y + star.unit.y * area.height();
    }
}

fn drive_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let delta = time.delta_secs();
    for (entity, mut particle, mut transform, mut sprite) in &mut particles {
        particle.life -= delta;
        if particle.life <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }

        particle.velocity.y += particle.gravity * delta;
        let step = particle.velocity * delta;
        transform.translation.x += step.x;
        transform.translation.y += step.y;
        transform.rotate_z(particle.spin * delta);

        let left = particle.life / particle.max_life;
        sprite.color = sprite.color.with_alpha(left.min(1.0));
    }
}
