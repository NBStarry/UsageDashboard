use crate::models::AppConfig;
use crate::paths;
use std::fs;

fn config_path() -> std::path::PathBuf {
    paths::config_dir().join("config.json")
}

pub fn load() -> AppConfig {
    if let Ok(data) = fs::read(config_path()) {
        if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&data) {
            return cfg;
        }
    }
    let def = AppConfig::default();
    let _ = save(&def);
    def
}

pub fn save(cfg: &AppConfig) -> std::io::Result<()> {
    let dir = paths::config_dir();
    fs::create_dir_all(&dir)?;
    let data = serde_json::to_vec_pretty(cfg).expect("serialize config");
    fs::write(config_path(), data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn load_writes_default_when_missing_then_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        let c = load();
        assert_eq!(c.services.len(), 3);
        assert!(crate::paths::config_dir().join("config.json").exists());

        let mut next = c.clone();
        next.refresh_seconds = 120;
        save(&next).unwrap();
        assert_eq!(load().refresh_seconds, 120);
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
