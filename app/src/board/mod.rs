mod bottle;

use bevy::prelude::*;

use crate::art::Art;
use crate::geometry::BoardGeometry;
use crate::theme;
use crate::view::BoardView;

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (spawn_board, (bottle::sync_items, bottle::sync_surfaces)).chain(),
        )
        .add_systems(
            Update,
            (bottle::sync_items, bottle::sync_surfaces).run_if(resource_changed::<BoardView>),
        );
    }
}

#[derive(Component)]
pub struct BoardRoot;

fn spawn_board(mut commands: Commands, art: Res<Art>, view: Res<BoardView>) {
    let geometry = BoardGeometry::new(view.slots());
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
