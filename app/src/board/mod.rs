mod bottle;
mod decor;
mod feature;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::anim::{Flow, Play};
use crate::art::Art;
use crate::geometry::BoardGeometry;
use crate::hud::{NavAction, NavRequest};
use crate::input::Selection;
use crate::rng::Pcg32;
use crate::session::Session;
use crate::theme;
use crate::view::BoardView;

pub use bottle::{Fluids, HitTarget};

const SHAKE_SEED: u64 = 0x5AFE_5EED;
const DECOR_SEED: u64 = 0x1CE_5EED;

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        let view = app
            .world()
            .get_resource::<BoardView>()
            .expect("SessionPlugin runs before BoardPlugin");
        let geometry = BoardGeometry::new(view.slots());

        app.insert_resource(geometry)
            .insert_resource(ShakeRng(Pcg32::new(SHAKE_SEED, 1)))
            .insert_resource(decor::DecorRng(Pcg32::new(DECOR_SEED, 1)))
            .init_resource::<Fluids>()
            .add_systems(Startup, spawn_board)
            .add_systems(
                Update,
                apply_nav
                    .in_set(Play::Input)
                    .after(crate::hud::handle_nav)
                    .after(crate::input::handle_click),
            )
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
                        feature::sync_questions,
                        feature::sync_bands,
                        feature::sync_lids,
                        feature::sync_plugs,
                        feature::sync_key_badges,
                        feature::sync_tags,
                        decor::sync_ice,
                        decor::sync_curtains,
                        decor::sync_safes,
                        decor::sync_safe_counters,
                        decor::sync_doors,
                        decor::sync_keys,
                        decor::sync_color_curtains,
                    ),
                )
                    .chain()
                    .in_set(Play::Apply),
            );
    }
}

#[derive(Component)]
pub struct BoardRoot;

#[derive(Resource)]
struct ShakeRng(Pcg32);

fn spawn_board(
    mut commands: Commands,
    art: Res<Art>,
    view: Res<BoardView>,
    geometry: Res<BoardGeometry>,
) {
    build(&mut commands, &art, &view, &geometry);
}

fn build(commands: &mut Commands, art: &Art, view: &BoardView, geometry: &BoardGeometry) {
    let root = commands
        .spawn((
            BoardRoot,
            Transform::from_xyz(0.0, 0.0, theme::Z_BOARD),
            Visibility::default(),
        ))
        .id();

    let colors = decor::lock_colors(view);
    for bottle in &view.bottles {
        bottle::spawn(commands, root, art, geometry, &colors, bottle);
    }
    decor::spawn(commands, root, art, geometry, view, &colors);
}

/// Everything a level change or a history step touches.
#[derive(SystemParam)]
struct Board<'w, 's> {
    view: ResMut<'w, BoardView>,
    geometry: ResMut<'w, BoardGeometry>,
    fluids: ResMut<'w, Fluids>,
    flow: ResMut<'w, Flow>,
    selection: ResMut<'w, Selection>,
    roots: Query<'w, 's, Entity, With<BoardRoot>>,
}

fn apply_nav(
    mut commands: Commands,
    art: Res<Art>,
    mut request: ResMut<NavRequest>,
    mut session: ResMut<Session>,
    mut shake: ResMut<ShakeRng>,
    mut board: Board,
) {
    let Some(action) = request.take() else {
        return;
    };

    match action {
        NavAction::Rewind | NavAction::Forward => {
            let stepped = match action {
                NavAction::Rewind => session.rewind(),
                _ => session.forward(),
            };
            if !stepped {
                return;
            }
            *board.view = session.view();
            board.selection.clear();
            for bottle in &board.view.bottles {
                let sign = if shake.0.next_u32().is_multiple_of(2) {
                    1.0
                } else {
                    -1.0
                };
                board.fluids.kick(bottle.id, sign * theme::FLUID_NAV_KICK);
            }
        }
        NavAction::Next => {
            if !session.advance() {
                return;
            }
            for root in &board.roots {
                commands.entity(root).despawn();
            }
            *board.view = session.view();
            *board.geometry = BoardGeometry::new(board.view.slots());
            *board.fluids = Fluids::default();
            board.selection.clear();
            board.flow.restart_intro();
            build(&mut commands, &art, &board.view, &board.geometry);
        }
    }
}
