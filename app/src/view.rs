#![allow(dead_code)]

use std::range::{Range, RangeInclusive};

use bevy::prelude::*;
use water_sort_core::layout::Layout;
use water_sort_core::state::{BOTTLE_COUNT, Move as CoreMove, Pours, State, to_index};

use crate::geometry::Slot;

/// Upper bound for arrays indexed by bottle id; core reserves index 0.
pub const MAX_BOTTLES: usize = BOTTLE_COUNT;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemView {
    pub color: u8,
    pub hidden: bool,
    pub locked: bool,
    /// Index of the lock group this item unlocks, `0` when it carries no key.
    pub key: u16,
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

    pub fn slot(&self) -> Slot {
        Slot {
            lines: self.lines,
            repr_column: self.repr_column,
            capacity: self.capacity,
        }
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
    pub pours: Option<Pours>,
}

impl BoardView {
    pub fn build(state: &State, layout: &Layout) -> Self {
        let bottles = state
            .bottles()
            .map(|id| bottle(state, layout, id))
            .collect();

        let pours = state.pours();

        Self {
            bottles,
            frozen_ranges: state.get_frozen_ranges().collect(),
            curtain_ranges: state.get_curtain_ranges().collect(),
            lock_groups: state.get_lock_group_ranges().collect(),
            solved: state.solved(),
            stuck: pours.is_some_and(|pours| pours.is_empty()),
            pours,
        }
    }

    pub fn can_pour(&self, from: u8, to: u8) -> bool {
        self.pours.is_some_and(|pours| pours.has(from, to))
    }

    pub fn get(&self, id: u8) -> &BottleView {
        self.bottles
            .iter()
            .find(|bottle| bottle.id == id)
            .expect("bottle is part of the board")
    }

    pub fn slots(&self) -> impl Iterator<Item = Slot> {
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
            key: match state.get_item_has_key(id, item) {
                true => to_index(id, item),
                false => 0,
            },
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

/// Everything the animation needs to know about a pour that has already been
/// applied to the state.
#[derive(Clone, Debug)]
pub struct MovePlan {
    pub from: u8,
    pub to: u8,
    pub count: u8,
    pub color: u8,
    pub from_height_before: u8,
    pub to_height_before: u8,
    pub finalize: Option<u8>,
    pub reveal_item: Option<(u8, u8)>,
    pub unlock_item: Option<(u8, u8)>,
    pub unplug: Option<u8>,
    pub plug_toggled: Vec<u8>,
    pub unfreeze: Option<RangeInclusive<u8>>,
    pub curtains_lifted: Vec<u8>,
    pub safe_ticks: Vec<u8>,
    pub safes_opened: Vec<u8>,
    pub key_used: Option<(u8, u8)>,
    pub unlock_run: Option<RangeInclusive<u8>>,
    pub color_curtains_lifted: Vec<u8>,
}

impl MovePlan {
    pub fn build(mov: &CoreMove, before: &BoardView, after: &BoardView) -> Self {
        let from = mov.from_bottle();
        let to = mov.to_bottle();

        let curtains_lifted = if mov.lift_curtains() {
            changed(before, after, |before, after| {
                before.behind_curtain && !after.behind_curtain
            })
        } else {
            Vec::new()
        };

        let (safe_ticks, safes_opened) = if mov.decrement_safe_counters() {
            let ticks = before
                .bottles
                .iter()
                .filter(|bottle| mov.decrement_safe_counter(bottle.id))
                .map(|bottle| bottle.id)
                .collect();
            let opened = changed(before, after, |before, after| {
                before.safe_counter > 0 && after.safe_counter == 0
            });
            (ticks, opened)
        } else {
            (Vec::new(), Vec::new())
        };

        let color_curtains_lifted = if mov.lift_color_curtains() {
            before
                .bottles
                .iter()
                .filter(|bottle| {
                    bottle.color_curtain.is_some() && mov.lift_color_curtain(bottle.id)
                })
                .map(|bottle| bottle.id)
                .collect()
        } else {
            Vec::new()
        };

        Self {
            from,
            to,
            count: mov.count(),
            color: mov.color(),
            from_height_before: before.get(from).items.len() as u8,
            to_height_before: before.get(to).items.len() as u8,
            finalize: mov.finalize_bottle(),
            reveal_item: mov.show_item(),
            unlock_item: mov.unlock_item(),
            unplug: mov.unplug_bottle(),
            plug_toggled: changed(before, after, |before, after| {
                before.plugged != after.plugged
            }),
            unfreeze: mov.unfreeze_run(),
            curtains_lifted,
            safe_ticks,
            safes_opened,
            key_used: mov.remove_key(),
            unlock_run: mov.unlock_run(),
            color_curtains_lifted,
        }
    }
}

fn changed(
    before: &BoardView,
    after: &BoardView,
    predicate: impl Fn(&BottleView, &BottleView) -> bool,
) -> Vec<u8> {
    before
        .bottles
        .iter()
        .filter(|bottle| predicate(bottle, after.get(bottle.id)))
        .map(|bottle| bottle.id)
        .collect()
}
