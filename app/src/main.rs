mod dev;

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
                        resolution: WindowResolution::new(440, 956),
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
        .insert_resource(ClearColor(Color::srgb(0.055, 0.067, 0.086)))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text2d::new("Hello, water sort!"),
        TextFont::from_font_size(32.0),
        TextColor(Color::srgb(0.88, 0.90, 0.94)),
        Transform::from_xyz(0.0, 180.0, 0.0),
    ));

    commands
        .spawn((
            Sprite::from_color(Color::srgb(0.20, 0.51, 0.91), Vec2::new(200.0, 200.0)),
            Transform::from_xyz(0.0, -60.0, 0.0),
            Pickable::default(),
        ))
        .observe(recolor_on_click);
}

fn recolor_on_click(
    click: On<Pointer<Click>>,
    mut sprites: Query<&mut Sprite>,
    mut count: Local<usize>,
) -> Result {
    const COLORS: [Color; 3] = [
        Color::srgb(0.20, 0.51, 0.91),
        Color::srgb(0.91, 0.30, 0.24),
        Color::srgb(0.18, 0.72, 0.42),
    ];

    *count += 1;
    sprites.get_mut(click.entity)?.color = COLORS[*count % COLORS.len()];
    Ok(())
}
