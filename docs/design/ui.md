Overview
========

This document describes the implementation of a water sort game. The game logic
is already implemented in core. The goal is to create the accompanying UI for a
PWA which should be optimized for the iPhone, built with Bevy (the app crate).

General
=======

The main game displays the columns in a grid as described in the Layout struct,
centered horizontally and vertically. The bottle sizes should be consistent for
all layouts, meaning consistent bottle and item heights and widths. Bottles
which span multiple lines can have a larger maximum capacity to accommodate
for the space between the lines, which is now filled in by the bottle. Bottle
positions should be stable, not changed by additional features like freezing
or curtains. Consecutive items which can be poured together should be merged by
not showing borders between these items with the same color.

At the start of the level, the bottles should shift in from the side quickly.
When clicking a bottle, the bottle should move up slightly with fluid
animations (when pouring from this bottle is possible via can_move_from,
otherwise do nothing). Then when clicking another bottle, a pouring animation
should be played (when pouring from the first bottle into the second bottle is
possible, otherwise select the second bottle if that is possible, otherwise do
nothing), where the first bottle moves over the second bottle. Clicking a
selected bottle again or clicking somewhere else deselects it, moving the
bottle back down.

In general all animations should be smoothened and run quickly (typically
hundreds of milliseconds for interactive animations). It should be possible
to cancel running animations by clicking somewhere else, which goes to the next
state immediately. When a bottle is clicked in this case, also select it. The
required animations can be obtained by querying the returned move data from
a successful pour and in some cases by creating a copy of the state (cheap)
before the pour and diffing with the state after the move.

When a bottle is finalized, a short swirling star animation should play, going
from the bottom of the bottle to the top, which should end with the bottle
being plugged with a cork. The cork plug is also the marker that a bottle was
solved.

When a level is completed, a short confetti animation should play with a level
completed text appearing on the screen, nicely animated. The level is slightly
darkened and no more interaction with the level is possible. Save the last
completed level index to the device. When the user presses the next level
button, load the next index or display a message saying no more levels, come
back later. The current level progress should not be persisted, so a page
reload will lose the progress of an individual level.

When no more moves are possible, slightly darken the level and display a
message saying no more moves. The navigation buttons should still be visible,
so the user can go back and try something else.

Navigation buttons are shown in a panel at the bottom of the screen, going
forwards and backwards when possible. This is enabled by the history API. Going
forwards or backwards should not play the standard animations. Just teleport
to the state with a tiny shaking of the fluids. No reset button to restart the
level should be implemented.

The level background should be dark purple, with small white star animations
appearing randomly.

No menu or settings screen is intended. On the first start of the game you get
directly thrown into the first level without any additional explanations. Only
show the level number at the top of the screen, one based. The level
description is ignored.

The game is portrait locked and delivered as a standalone PWA with no-bounce.
The bottom nav panel and top level indicator should leave enough space to not
collide with the home indicator and camera notch, the background can fill in
the dead space. No audio, haptics, or accessibility features are planned.

The style should be colorful and playful, always enhanced with fun animations,
including simple fluid animations when moving bottles, also for smaller
movements.

Assume the default feature set and environment settings for core.

Some of the core API functions can return the zero value for a bottle, color,
or key, which indicates absence of a value.

Additional Features
===================

Items which are hidden replace their color with black and show a question mark
on top. Additional item specific features are also invisible. The top item of a
bottle can never be hidden. Revealing a hidden item fades it in quickly.

Locked items are shown with a small metal border. When a locked item is on top,
also show a metal top for this item. When a bottle is shifted up for a move and
a locked item is on top, the metal cover should open up. The lock is consumed
when the item is poured out.

Immovable bottles have a bottom with small spiky rocks, showing this bottle is
locked in place.

Pluggable bottles have a small plug on top, either closing the bottle or
hanging to the side, which changes every turn dependent on the state. A corked
finalized bottle never renders a plug.

Frozen bottles stand in a block of ice with a crystalline top edge, covering
their lower part and spanning the whole range as one block, so the colours
above it stay readable. Unfreezing a range results in the ice exploding, which
happens when a bottle in the frozen range is solved.

Curtain ranges hide their underlying bottles, so they cannot be interacted
with. Lifting part of a curtain, exactly one bottle per curtain group per
finalized bottle from the highest index down, rolls it to the left. Both kinds
of curtain read as cloth rather than as rectangles: waved and hemmed edges,
with the outer corners rounded.

Safes hide a bottle, so it cannot be interacted with. The counter is shown on
top of the relevant safe, decrementing when solving any bottle. A zero safe
counter means the safe has been unlocked or there was no safe. Equal safe
counters which are located right next to each other in the layout and only span
a single line are grouped together behind a single larger safe. The groups can
never change, as counters are decremented in lockstep.

Locked groups have a colored lock, matching the colored key (different to the
item color, select a color per key/lock combo at the start of the level)
displayed on the relevant item. When unlocking the group, the key is shown
flying into the lock, opening the door. Bottles in locked groups are hidden
and cannot be interacted with before the door is opened.

Colored bottles have a small color tag which hangs to the side of the bottle.
Make sure this does not collide with a possible plug. This is an intake only
filter, pouring out is unrestricted and other colors can be in this bottle at
the start.

Color curtains hide a bottle with a cloth until that color is solved. The
matching color is displayed as a small icon on top of the cloth. Color curtains
are never grouped together. Include an animation when lifting the curtain.
