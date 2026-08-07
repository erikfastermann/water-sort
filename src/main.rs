/*
Search:

- Check current state is final, if yes return depth 0
- Store current state and check if known already, 3 possibilities:
  - First time seen -> Store as invalid
  - Not first time and stored invalid -> Return invalid
    (catches loops and dead states at once)
  - Not first time and stored with dist -> Return stored dist
    (when a move was found which reaches the end, we always store the returned
    dist plus one, because they might be invalid currently or the score has improved,
    don't store if the current is worse)
  - Question: How to store this efficiently? Maybe it would be better to just
    check for loops and ignore transpositions for now, this would be simple by
    hashing the state in the current layer, and then comparing equals with that
    state (what is enough to store here?), but this also means we need to
    decouple state from history
    -> just do loops now
- Find possible moves in layer, Bitmap N x N-1,
  store in Vec at index for current search depth
  (allocate once, also gives max depth for search)
  -> because fixed width, we can allocate on the stack
- Apply each move
- Also track current best score (which is equal to max depth)
  -> if we are at this depth, we can return immediately,
     because we found a solution which is equally good or better already,
     unless we want to enumerate all solutions, but this might be harder

- if we store the state for each depth, we have to mem copy a lot,
  we only need to store this to compare if we have seen this state before,
  storing just the hash should be fine usually, but hash collisions can happen,
  but are really rare
- what are alternatives to not store the full state?
- the case where a move is detected as duplicate, but is actually not is
  annoying, but we probably accept this small risk
- Zobrist hashing can be done later

- we have one state, ability to compute the moves and what changes would need
  to be applied and how to undo a move again

Three ideas:

(1) Standard Rust program
(2) Rust program where the input is compiled in
(3) Minizinc Program driven by something else

We want:

- Load/Store state in memory and on disk
- Get legal moves
- (Un-)apply move
- Move history with navigation
- Solver to check if config/state is solvable
  (satisfiable and shortest solution)
- More tooling which could be used to create levels,
  possibly driven by an LLM
- Debugging infrastructure

(3) might be hard to achieve, with slow startup times
just to compute the next moves (100ms minimum), although
the logic for (un-)applying moves I imagine would be quite
nice, maybe there is some nicer option than Minizinc,
directly driven from Rust, which can be meta-programmed easily.
We always have to ship a complete Minizinc, also annoying,
not too embeddable probably.

Performance with (3) is also questionable, specially with so
many requirements, although partially disabling them sounds
easy. In some cases the performance might also be better,
too hard to tell currently without trying it. Debugging is
often annoying though.

(1) + (2) would be interesting and probably the most versatile.
Corner case bugs could be tricky and everything has to be done
manually. But we get maximum control and would also be an
interesting experiment. Still questionable how good the logic
sharing would work, needs to be seen.

Could inject into state multiple const vars: bottle count (used
instead of associated bottle count if less than MAX), max bottle
height (probably as a total for all bottles, ut for individual
bottles this could be even more powerful), boolean feature flags.
*/

use crate::bits::Bits;

mod bits;

const MAX_BOTTLE_SIZE: usize = 15;

const BOTTLE_SIZE: usize = MAX_BOTTLE_SIZE + 1;

const BOTTLE_SIZE_BITS: usize = 4;

/// The zero value is reserved.
const MAX_BOTTLE_COUNT: usize = 31;

const BOTTLE_COUNT: usize = MAX_BOTTLE_COUNT + 1;

const BOTTLE_COUNT_BITS: usize = 5;

/// The zero value is reserved.
const MAX_COLORS: usize = 15;

const COLOR_BITS: usize = 4;

/// Represents the current game state without history.
///
/// At most one of the following can be chosen per bottle: curtain, safe, or
/// lock.
struct State {
    /// Must be less than the maximum amount of bottles.
    bottle_count: u8,

    content: Bits<{ BOTTLE_SIZE * BOTTLE_COUNT * COLOR_BITS }>,
    height: Bits<{ BOTTLE_COUNT * BOTTLE_SIZE_BITS }>,
    capacity: Bits<{ BOTTLE_COUNT * BOTTLE_SIZE_BITS }>,

    // The user cannot see this color item. Only relevant for the solver when
    // pouring out of this bottle, items connected by color are only poured
    // until the first invisible item.
    item_visible: Bits<{ BOTTLE_SIZE * BOTTLE_COUNT }>,

    // The current item can only be moved out of this bottle and only one at
    // a time. Pouring into this bottle is not allowed if the top item is
    // locked. The lock disappears once the item has been removed.
    item_locked: Bits<{ BOTTLE_SIZE * BOTTLE_COUNT }>,

    // Can only fill into this bottle, never pour out of it.
    bottle_movable: Bits<{ BOTTLE_COUNT }>,

    // Each move of any bottle, all plugged bottles are unplugged and
    // vice-versa.
    plugged_general: Bits<{ BOTTLE_COUNT }>,
    plugged_state: Bits<{ BOTTLE_COUNT }>,

    // A frozen bottle can only be filled into, until it is unfrozen.
    // Unfreezing happens once a bottle inside a frozen group is solved.
    //
    // Each bottle has an index of the group which this bottle is frozen with,
    // or zero if the bottle was never frozen. If a bottle is marked as frozen
    // once, this never changes, just update the frozen bit. As freezing a
    // bottle is only sensible if it has at least one partner, the maximum
    // number of frozen groups should be 16 to be solvable.
    frozen_group: [u8; BOTTLE_COUNT],
    frozen: Bits<{ BOTTLE_COUNT }>,

    // A bottle behind a curtain is not visible and cannot be interacted with.
    // When any other bottle is solved, each curtain group unlocks a bottle
    // in a defined order. Each bottle, except for two, can be behind a
    // curtain at the same time to be solvable and a curtain group can consist
    // of only a single bottle.
    curtain_order: [u8; BOTTLE_COUNT],
    // The zero offset is reserved to mark the end of valid groups.
    group_offset: [u8; BOTTLE_COUNT],
    behind_curtain: Bits<{ BOTTLE_COUNT }>,

    // A bottle in a safe, meaning a counter greater than zero, is not visible
    // and cannot be interacted with. Each time any bottle is solved, all
    // non-zero safe counters are decremented by one. Safes could be grouped
    // in the UI, but this is not relevant for the state representation. Each
    // bottle, except for two, could be stored in a safe at the same time to
    // be solvable.
    safe_counter: [u8; BOTTLE_COUNT],

    // All bottles in a locked group cannot be interacted with. One color item
    // has the associated key for each lock group. When this item is at the
    // top of a bottle, the group is unlocked. When multiple items can be
    // poured at the same time, the key must always be the top item.
    lock_group: [u8; BOTTLE_COUNT],
    item_has_lock: Bits<{ BOTTLE_SIZE * BOTTLE_COUNT }>,
    group_key_location: [u8; BOTTLE_COUNT],
    bottle_locked: Bits<{ BOTTLE_COUNT }>,

    // Only items of a specific color can be poured into this bottle, or zero,
    // if all colors are allowed. Storing other colors in such a bottle is
    // allowed.
    bottle_color: Bits<{ BOTTLE_COUNT * COLOR_BITS }>,

    // The Bottle cannot be interacted with, until that specific color is
    // solved. The zero color implies that the bottle is not hidden by a
    // color curtain.
    color_curtain: Bits<{ BOTTLE_COUNT * COLOR_BITS }>,
}

// TODO:
// - Bottle With Counter, Refilled When Empty With Counter Greater Than Zero

fn main() {}
