mod anim;
mod art;
mod board;
mod dev;
mod fx;
mod geometry;
mod input;
mod raster;
mod rng;
mod session;
mod theme;
mod view;

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Water Sort".into(),
                        canvas: Some("#game".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        resolution: WindowResolution::new(
                            theme::CANVAS_W as u32,
                            theme::CANVAS_H as u32,
                        ),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(dev::DevPlugin)
        .add_plugins(art::ArtPlugin)
        .add_plugins(fx::BackgroundPlugin)
        .add_plugins(session::SessionPlugin)
        .add_plugins(anim::AnimPlugin)
        .add_plugins(board::BoardPlugin)
        .add_plugins(input::InputPlugin)
        .add_systems(Startup, spawn_camera)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: theme::CANVAS_W,
                min_height: theme::CANVAS_H,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}
