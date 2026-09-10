mod bottle;

use bevy::prelude::*;

use crate::anim::Play;
use crate::art::Art;
use crate::geometry::BoardGeometry;
use crate::theme;
use crate::view::BoardView;

pub use bottle::{Fluids, HitTarget};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        let view = app
            .world()
            .get_resource::<BoardView>()
            .expect("SessionPlugin runs before BoardPlugin");
        let geometry = BoardGeometry::new(view.slots());

        app.insert_resource(geometry)
            .init_resource::<Fluids>()
            .add_systems(Startup, spawn_board)
            .add_systems(
                Update,
                (
                    bottle::apply_offsets,
                    bottle::apply_tilts,
                    bottle::apply_fluid,
                    (
                        bottle::sync_items,
                        bottle::sync_surfaces,
                        bottle::apply_corks,
                        bottle::apply_halos,
                    ),
                )
                    .chain()
                    .in_set(Play::Apply),
            );
    }
}

#[derive(Component)]
pub struct BoardRoot;

fn spawn_board(
    mut commands: Commands,
    art: Res<Art>,
    view: Res<BoardView>,
    geometry: Res<BoardGeometry>,
) {
    let root = commands
        .spawn((
            BoardRoot,
            Transform::from_xyz(0.0, 0.0, theme::Z_BOARD),
            Visibility::default(),
        ))
        .id();

    for view in &view.bottles {
        bottle::spawn(&mut commands, root, &art, &geometry, view);
    }
}
