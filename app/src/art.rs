use bevy::prelude::*;

use crate::raster::{Raster, ellipse, radial, radial_inverse, rect, sparkle};
use crate::rng::Pcg32;

const SCALE: u32 = 2;
const NOISE_SEED: u64 = 0xA17_5EED;
const GRADIENT_NOISE: f32 = 0.02;

#[derive(Resource)]
pub struct Art {
    pub glow: Handle<Image>,
    pub vignette: Handle<Image>,
    pub star: Handle<Image>,
}

pub struct ArtPlugin;

impl Plugin for ArtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, build_art);
    }
}

fn build_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut rng = Pcg32::new(NOISE_SEED, 1);
    commands.insert_resource(Art {
        glow: glow(&mut images, &mut rng),
        vignette: vignette(&mut images, &mut rng),
        star: star(&mut images),
    });
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
