use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::art::{Art, ITEM_SURFACE_H, ITEM_W, glass_size};
use crate::geometry::{BoardGeometry, check_capacity};
use crate::theme;
use crate::view::{BoardView, BottleView};

/// Fixed home position of a bottle; never moves.
#[derive(Component)]
pub struct BottleSlot;

/// Carries the lift and pour offsets away from the home position.
#[derive(Component)]
pub struct BottleRoot;

/// Sits at the mouth so a rotation on it pivots around the rim; [`BottleArt`]
/// undoes the offset for the sprites below.
#[derive(Component)]
pub struct BottleTilt;

#[derive(Component)]
pub struct BottleArt;

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

pub fn spawn(
    commands: &mut Commands,
    board: Entity,
    art: &Art,
    geometry: &BoardGeometry,
    view: &BottleView,
) {
    let lines = view.line_span();
    check_capacity(view.capacity, lines);

    let bounds = geometry.bottle_rect(view.lines, view.repr_column);
    let center = bounds.center();
    let pivot = geometry.mouth(bounds) - center;

    let slot = commands
        .spawn((
            BottleSlot,
            Transform::from_xyz(center.x, center.y, 0.0),
            Visibility::default(),
            ChildOf(board),
        ))
        .id();
    let root = commands
        .spawn((
            BottleRoot,
            Transform::default(),
            Visibility::default(),
            ChildOf(slot),
        ))
        .id();
    let tilt = commands
        .spawn((
            BottleTilt,
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

    let glass = Some(glass_size(lines));
    commands.spawn((
        Sprite {
            image: art.glass_back[usize::from(lines) - 1].clone(),
            color: theme::GLASS_INTERIOR,
            custom_size: glass,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_GLASS_BACK),
        Pickable::IGNORE,
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
                Pickable::IGNORE,
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
            Pickable::IGNORE,
            ChildOf(slot),
        ));
    }

    commands.spawn((
        Sprite {
            image: art.glass_front[usize::from(lines) - 1].clone(),
            color: theme::GLASS_RIM,
            custom_size: glass,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_GLASS_FRONT),
        Pickable::IGNORE,
        ChildOf(holder),
    ));
}

pub fn sync_items(
    view: Res<BoardView>,
    mut slots: Query<(&ItemSlot, &mut Sprite, &mut Visibility)>,
) {
    for (slot, mut sprite, mut visibility) in &mut slots {
        let items = &view.get(slot.bottle).items;
        match items.get(usize::from(slot.index)) {
            Some(item) => {
                *visibility = Visibility::Inherited;
                sprite.color = theme::item_color(item.color);
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// The rounded liquid surface only belongs on the topmost item; it stays a
/// child of that item so it is hidden and moved along with it.
pub fn sync_surfaces(
    view: Res<BoardView>,
    mut surfaces: Query<(&ItemSurface, &mut Sprite, &mut Visibility)>,
) {
    for (surface, mut sprite, mut visibility) in &mut surfaces {
        let items = &view.get(surface.bottle).items;
        let index = usize::from(surface.index);
        *visibility = if items.len() == index + 1 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(item) = items.get(index) {
            sprite.color = theme::item_color(item.color);
        }
    }
}
