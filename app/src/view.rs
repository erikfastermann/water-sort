#![allow(dead_code)]

use std::range::{Range, RangeInclusive};

use bevy::prelude::*;
use water_sort_core::layout::Layout;
use water_sort_core::state::State;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemView {
    pub color: u8,
    pub hidden: bool,
    pub locked: bool,
    pub has_key: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BottleView {
    pub id: u8,
    pub lines: Range<u8>,
    pub repr_column: u8,
    pub capacity: u8,
    pub items: Vec<ItemView>,
    pub finalized: bool,
    pub immovable: bool,
    pub pluggable: bool,
    pub plugged: bool,
    pub frozen: bool,
    pub behind_curtain: bool,
    pub safe_counter: u8,
    pub locked: bool,
    pub filter_color: Option<u8>,
    pub color_curtain: Option<u8>,
    pub can_move_from: bool,
    pub interactable: bool,
}

impl BottleView {
    pub fn line_span(&self) -> u8 {
        self.lines.end - self.lines.start
    }

    pub fn slot(&self) -> (Range<u8>, u8) {
        (self.lines, self.repr_column)
    }
}

#[derive(Clone, Resource)]
pub struct BoardView {
    pub bottles: Vec<BottleView>,
    pub frozen_ranges: Vec<RangeInclusive<u8>>,
    pub curtain_ranges: Vec<RangeInclusive<u8>>,
    pub lock_groups: Vec<(u16, RangeInclusive<u8>)>,
    pub solved: bool,
    pub stuck: bool,
}

impl BoardView {
    pub fn build(state: &State, layout: &Layout) -> Self {
        let bottles = state
            .bottles()
            .map(|id| bottle(state, layout, id))
            .collect();

        Self {
            bottles,
            frozen_ranges: state.get_frozen_ranges().collect(),
            curtain_ranges: state.get_curtain_ranges().collect(),
            lock_groups: state.get_lock_group_ranges().collect(),
            solved: state.solved(),
            stuck: state.pours().is_some_and(|pours| pours.is_empty()),
        }
    }

    pub fn get(&self, id: u8) -> &BottleView {
        self.bottles
            .iter()
            .find(|bottle| bottle.id == id)
            .expect("bottle is part of the board")
    }

    pub fn slots(&self) -> impl Iterator<Item = (Range<u8>, u8)> {
        self.bottles.iter().map(BottleView::slot)
    }
}

fn bottle(state: &State, layout: &Layout, id: u8) -> BottleView {
    let (lines, repr_column) = layout
        .bottle_position(id)
        .expect("every bottle has a layout position");

    let items = (0..state.get_height(id))
        .map(|item| ItemView {
            color: state.get_color(id, item),
            hidden: state.get_item_hidden(id, item),
            locked: state.get_item_locked(id, item),
            has_key: state.get_item_has_key(id, item),
        })
        .collect();

    let behind_curtain = state.get_behind_curtain(id);
    let safe_counter = state.get_safe_counter(id);
    let locked = state.get_bottle_locked(id);
    let color_curtain = state
        .get_color_curtain_active(id)
        .then(|| state.get_color_curtain(id))
        .filter(|color| *color != 0);

    BottleView {
        id,
        lines,
        repr_column,
        capacity: state.get_capacity(id),
        items,
        finalized: state.get_bottle_finalized(id),
        immovable: state.get_bottle_immovable(id),
        pluggable: state.get_pluggable_bottle(id),
        plugged: state.get_bottle_plugged(id),
        frozen: state.get_frozen(id),
        behind_curtain,
        safe_counter,
        locked,
        filter_color: Some(state.get_bottle_color(id)).filter(|color| *color != 0),
        color_curtain,
        can_move_from: state.can_move_from(id),
        interactable: !behind_curtain && safe_counter == 0 && !locked && color_curtain.is_none(),
    }
}
