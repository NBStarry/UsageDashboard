#![allow(dead_code)]

use std::path::PathBuf;

fn override_root() -> Option<PathBuf> {
    std::env::var_os("USAGE_DASHBOARD_HOME").map(PathBuf::from)
}

pub fn config_dir() -> PathBuf {
    if let Some(r) = override_root() { return r.join("usage-bar"); }
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("usage-bar")
}

pub fn cache_dir() -> PathBuf {
    if let Some(r) = override_root() { return r.join("usage-dashboard"); }
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("usage-dashboard")
}

pub fn home_dir() -> Option<PathBuf> {
    if let Some(r) = override_root() { return Some(r); }
    dirs::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn config_dir_under_override_root() {
        std::env::set_var("USAGE_DASHBOARD_HOME", "C:\\tmp\\udtest");
        assert!(config_dir().ends_with("usage-bar"));
        assert!(cache_dir().ends_with("usage-dashboard"));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
