General
=======

The core crate holds the game state, layout, history and solver. The cli crate
runs a search over a starting state json. The app crate is the Bevy UI, shipped
as a PWA.

All of the state handling should live in core.

Don't edit core. If something is missing from core or not part of the public
API, flag your findings. Don't reimplement functionality from core inside app.

You are not allowed to add dependencies on your own. Propose such changes first
for review and discussion.

Avoid comments, only where behavior is surprising.

The UI specification lives in docs/design/ui.md. The Bevy UI should stay in
sync.

Bevy is pinned to 0.19, which differs substantially from older releases. Check
docs.rs for the exact version and latest APIs instead of recalling.

Commands
========

    cargo test --workspace
    cargo clippy --workspace --all-targets
    cargo fmt

    cargo run -p water-sort-app --features dev
    trunk build --config app/Trunk.toml

The dev feature enables dynamic linking for faster native iteration. It links
against libbevy_dylib.so, so that binary is not portable.

    trunk serve --config app/Trunk.toml --cargo-profile dev --release false

Trunk.toml defaults to the optimized web profile, which takes about three
minutes per change. The flags above drop that to twenty seconds for iterating
in a browser.

Changing a profile, feature or rustflag rebuilds Bevy from scratch, six to
eleven minutes. A build can trigger the OOM killer, so cap it:

    CARGO_BUILD_JOBS=4 systemd-run --user --scope -p MemoryMax=9G \
        -p MemorySwapMax=0 cargo build -p water-sort-app

Core has to keep building with any subset of its features. The command below
only checks the no-feature case, individual features need their own runs:

    cargo check -p water-sort-core --no-default-features

Dev channel
===========

A native build with WATER_SORT_DEV_DIR set polls <dir>/commands and runs every
appended line:

    shot [name]   save a screenshot to <dir>, default shot-<n>.png
    quit          exit the app

Append one command at a time and wait a moment between them. Screenshots are
saved over the following frames, so a quit in the same batch cancels them, and
two shots in one batch only produce one file.

The read offset starts at zero each run, so a reused dir replays its commands
file from the top. Use a fresh dir per run.

Use this to look at UI changes instead of assuming they work.
