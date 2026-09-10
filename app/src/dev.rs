use bevy::prelude::*;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        native::build(app);
        #[cfg(target_arch = "wasm32")]
        let _ = app;
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        env,
        fs::{self, File},
        io::{BufRead, BufReader, Seek, SeekFrom},
        path::PathBuf,
    };

    use bevy::{
        ecs::system::SystemParam,
        prelude::*,
        render::view::screenshot::{Screenshot, save_to_disk},
    };

    use crate::board::HitTarget;
    use crate::fx::Backdrop;
    use crate::hud::{NavAction, NavButton};
    use crate::input::PendingClick;

    const DIR_VAR: &str = "WATER_SORT_DEV_DIR";

    pub fn build(app: &mut App) {
        let Some(dir) = env::var_os(DIR_VAR).map(PathBuf::from) else {
            return;
        };

        if let Err(err) = fs::create_dir_all(&dir) {
            error!("cannot use {}: {err}", dir.display());
            return;
        }

        info!("dev channel: {}", dir.join("commands").display());
        app.insert_resource(DevChannel {
            commands: dir.join("commands"),
            dir,
            offset: 0,
        })
        .add_systems(Update, poll_commands.before(crate::anim::Play::Input));
    }

    /// Everything `tap` can address.
    #[derive(SystemParam)]
    struct TapTargets<'w, 's> {
        bottles: Query<'w, 's, (Entity, &'static HitTarget)>,
        nav: Query<'w, 's, (Entity, &'static NavButton)>,
        backdrop: Query<'w, 's, Entity, With<Backdrop>>,
    }

    impl TapTargets<'_, '_> {
        fn resolve(&self, argument: &str) -> Option<Entity> {
            let action = match argument {
                "back" => Some(NavAction::Rewind),
                "fwd" => Some(NavAction::Forward),
                "next" => Some(NavAction::Next),
                _ => None,
            };
            match (argument.parse::<u8>(), action) {
                (Ok(bottle), _) => self
                    .bottles
                    .iter()
                    .find(|(_, target)| target.id == bottle)
                    .map(|(entity, _)| entity),
                (_, Some(action)) => self
                    .nav
                    .iter()
                    .find(|(_, button)| button.action == action)
                    .map(|(entity, _)| entity),
                _ => self.backdrop.iter().next(),
            }
        }
    }

    #[derive(Resource)]
    struct DevChannel {
        dir: PathBuf,
        commands: PathBuf,
        offset: u64,
    }

    fn poll_commands(
        mut channel: ResMut<DevChannel>,
        mut commands: Commands,
        mut exit: MessageWriter<AppExit>,
        mut pending: ResMut<PendingClick>,
        targets: TapTargets,
        mut shots: Local<usize>,
    ) {
        let Ok(mut file) = File::open(&channel.commands) else {
            return;
        };
        if file.seek(SeekFrom::Start(channel.offset)).is_err() {
            return;
        }

        let mut reader = BufReader::new(file);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if !line.ends_with('\n') {
                        break;
                    }
                    channel.offset += read as u64;
                }
            }

            let mut words = line.split_whitespace();
            match words.next() {
                None => continue,
                Some("shot") => {
                    let name = words
                        .next()
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("shot-{}.png", *shots));
                    *shots += 1;
                    let path = channel.dir.join(name);
                    info!("screenshot: {}", path.display());
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(path));
                }
                Some("tap") => {
                    let argument = words.next().unwrap_or("bg");
                    match targets.resolve(argument) {
                        Some(entity) => pending.set(entity),
                        None => warn!("no tap target for: {argument}"),
                    }
                }
                Some("quit") => {
                    exit.write(AppExit::Success);
                }
                Some(other) => warn!("unknown dev command: {other}"),
            }
        }
    }
}
