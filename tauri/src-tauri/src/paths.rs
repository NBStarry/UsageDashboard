use std::path::PathBuf;
use std::sync::OnceLock;

// 移动端在 setup() 里注入的可写沙盒根目录(Tauri app_config_dir)。
// 桌面端不设置,继续走 dirs::*(并能读 ~/.claude 等真实凭证)。
static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

// 移动端 dirs::config_dir() 会落到 /.config 这类只读路径(写入报 EROFS)。
// setup() 解析出 app 沙盒目录后调用本函数注入,之后所有路径都改用它。
// 仅 #[cfg(mobile)] 的 setup 调用它,桌面端构建里是 dead code,显式 allow。
#[cfg_attr(not(mobile), allow(dead_code))]
pub fn set_base_dir(p: PathBuf) {
    let _ = BASE_DIR.set(p);
}

// 根目录优先级:运行期注入(移动端沙盒) > 环境变量覆盖(测试/桌面调试) > 无。
fn root_override() -> Option<PathBuf> {
    if let Some(p) = BASE_DIR.get() {
        return Some(p.clone());
    }
    std::env::var_os("USAGE_DASHBOARD_HOME").map(PathBuf::from)
}

pub fn config_dir() -> PathBuf {
    if let Some(r) = root_override() { return r.join("usage-bar"); }
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("usage-bar")
}

pub fn cache_dir() -> PathBuf {
    if let Some(r) = root_override() { return r.join("usage-dashboard"); }
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("usage-dashboard")
}

pub fn home_dir() -> Option<PathBuf> {
    if let Some(r) = root_override() { return Some(r); }
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
