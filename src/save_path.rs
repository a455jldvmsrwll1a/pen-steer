use std::{path::PathBuf, str::FromStr};

use directories::ProjectDirs;

pub fn save_path() -> PathBuf {
    save_dir().join("pen-steer.conf")
}

pub fn save_dir() -> PathBuf {
    if let Some(override_path) = std::env::var_os("CONFIG_PATH") {
        return PathBuf::from(override_path);
    }

    let Some(dirs) = ProjectDirs::from("", "", "pen-steer") else {
        return std::env::current_dir().unwrap_or_else(|_| {
            PathBuf::from_str(".").expect("hardcoded string should be a valid path")
        });
    };

    dirs.config_local_dir().to_owned()
}
