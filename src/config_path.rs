use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    if let Some(path) = appimage_config_path() {
        return path;
    }
    if let Some(path) = executable_config_path() {
        return path;
    }
    current_dir_config_path()
}

fn appimage_config_path() -> Option<PathBuf> {
    let appimage = std::env::var("APPIMAGE").ok()?;
    PathBuf::from(appimage)
        .parent()
        .map(|p| p.join("config.yaml"))
}

fn executable_config_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|p| p.join("config.yaml"))
}

fn current_dir_config_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config.yaml")
}
