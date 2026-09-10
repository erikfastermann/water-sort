//! Test-only walks over core's own move generator. Nothing here belongs in the
//! app at runtime; it exists so a test can drive a level the way a player does.

use std::collections::HashSet;

use crate::session::Session;
use crate::view::BoardView;

type Fingerprint = Vec<(u8, Vec<u8>, u8, u8)>;

fn fingerprint(view: &BoardView) -> Fingerprint {
    view.bottles
        .iter()
        .map(|bottle| {
            let flags = u8::from(bottle.finalized)
                | u8::from(bottle.behind_curtain) << 1
                | u8::from(bottle.frozen) << 2
                | u8::from(bottle.plugged) << 3
                | u8::from(bottle.locked) << 4;
            (
                bottle.id,
                bottle.items.iter().map(|item| item.color).collect(),
                bottle.safe_counter,
                flags,
            )
        })
        .collect()
}

pub fn legal_pours(session: &Session) -> Vec<(u8, u8)> {
    let state = session.state();
    let Some(pours) = state.pours() else {
        return Vec::new();
    };
    let bottles: Vec<u8> = state.bottles().collect();
    let mut out = Vec::new();
    for from in bottles.iter().copied() {
        for to in bottles.iter().copied() {
            if from != to && pours.has(from, to) {
                out.push((from, to));
            }
        }
    }
    out
}

/// Depth-first walk that stops at the first solution, using the view as the
/// visited fingerprint.
pub fn solve(level: usize) -> Vec<(u8, u8)> {
    let mut session = Session::new(level).expect("level exists");
    let mut seen: HashSet<Fingerprint> = HashSet::new();
    let mut path = Vec::new();
    let mut stack = vec![legal_pours(&session).into_iter()];

    while let Some(options) = stack.last_mut() {
        if session.view().solved {
            return path;
        }
        match options.next() {
            None => {
                stack.pop();
                path.pop();
                session.history_mut().rewind();
            }
            Some((from, to)) => {
                if session.history_mut().pour(from, to).is_err() {
                    continue;
                }
                if !seen.insert(fingerprint(&session.view())) {
                    session.history_mut().rewind();
                    continue;
                }
                path.push((from, to));
                stack.push(legal_pours(&session).into_iter());
            }
        }
    }

    panic!("level {level} has no solution");
}

/// Visits every state reachable from the start of `level`, up to `cap` of them.
pub fn reachable(level: usize, cap: usize, mut visit: impl FnMut(&BoardView)) {
    let mut session = Session::new(level).expect("level exists");
    let mut seen: HashSet<Fingerprint> = HashSet::new();
    let view = session.view();
    seen.insert(fingerprint(&view));
    visit(&view);

    let mut stack = vec![legal_pours(&session).into_iter()];
    while let Some(options) = stack.last_mut() {
        if seen.len() >= cap {
            return;
        }
        match options.next() {
            None => {
                stack.pop();
                session.history_mut().rewind();
            }
            Some((from, to)) => {
                if session.history_mut().pour(from, to).is_err() {
                    continue;
                }
                let view = session.view();
                if !seen.insert(fingerprint(&view)) {
                    session.history_mut().rewind();
                    continue;
                }
                visit(&view);
                stack.push(legal_pours(&session).into_iter());
            }
        }
    }
}
