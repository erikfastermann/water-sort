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
        prelude::*,
        render::view::screenshot::{Screenshot, save_to_disk},
    };

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
        .add_systems(Update, poll_commands);
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
                Some("quit") => {
                    exit.write(AppExit::Success);
                }
                Some(other) => warn!("unknown dev command: {other}"),
            }
        }
    }
}
