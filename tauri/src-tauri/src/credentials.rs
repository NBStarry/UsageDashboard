#![allow(dead_code)]

use serde_json::Value;
use std::path::Path;

use crate::paths::{config_dir, home_dir};

// ─── Claude ───────────────────────────────────────────────────────────────────

pub fn claude_token() -> Option<String> {
    let home = home_dir()?;
    let path = home.join(".claude").join(".credentials.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    parse_claude_oauth(&raw)
}

fn parse_claude_oauth(raw: &str) -> Option<String> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let tok = v["claudeAiOauth"]["accessToken"].as_str()?;
    if tok.is_empty() { return None; }
    Some(tok.to_string())
}

// ─── Codex / GPT ──────────────────────────────────────────────────────────────

pub enum CodexCreds {
    Ok { access_token: String, account_id: String },
    MissingFile,
    ParseError,
    Incomplete,
}

pub fn codex_creds() -> CodexCreds {
    let home = match home_dir() {
        Some(h) => h,
        None => return CodexCreds::MissingFile,
    };
    let path = home.join(".codex").join("auth.json");
    if !path.exists() {
        return CodexCreds::MissingFile;
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return CodexCreds::ParseError,
    };
    let v: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return CodexCreds::ParseError,
    };

    let tokens = v.get("tokens");
    let access = tokens
        .and_then(|t| t["access_token"].as_str())
        .or_else(|| v["access_token"].as_str());
    let account = tokens
        .and_then(|t| t["account_id"].as_str())
        .or_else(|| v["account_id"].as_str());

    match (access, account) {
        (Some(a), Some(acc)) if !a.is_empty() && !acc.is_empty() => CodexCreds::Ok {
            access_token: a.to_string(),
            account_id: acc.to_string(),
        },
        _ => CodexCreds::Incomplete,
    }
}

// ─── New-API 兼容网关 ─────────────────────────────────────────────────────────

pub struct NewApiCredsData {
    pub base_url: String,
    pub access_token: String,
    pub user_id: i64,
    pub quota_per_unit: f64,
    pub currency: String,
}

pub enum NewApiCreds {
    Ok(NewApiCredsData),
    MissingFile(String),
    InvalidPath,
    Incomplete,
}

pub fn new_api_creds(file_name: &str) -> NewApiCreds {
    // Path validation: non-empty, not absolute, no ".." component
    let clean = file_name.trim();
    if clean.is_empty() {
        return NewApiCreds::InvalidPath;
    }
    if Path::new(clean).is_absolute() {
        return NewApiCreds::InvalidPath;
    }
    if clean.contains("..") {
        return NewApiCreds::InvalidPath;
    }

    let cfg = config_dir();
    let path = cfg.join(clean);

    if !path.exists() {
        return NewApiCreds::MissingFile(path.to_string_lossy().to_string());
    }

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return NewApiCreds::Incomplete,
    };
    let v: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return NewApiCreds::Incomplete,
    };

    let base_url = match v["baseUrl"].as_str() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return NewApiCreds::Incomplete,
    };
    let access_token = match v["accessToken"].as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return NewApiCreds::Incomplete,
    };

    // userId: tolerates number or string, defaults to 0
    let user_id: i64 = if let Some(n) = v["userId"].as_i64() {
        n
    } else if let Some(s) = v["userId"].as_str() {
        s.parse().unwrap_or(0)
    } else {
        0
    };

    // quotaPerUnit: defaults to 500000
    let quota_per_unit: f64 = if let Some(n) = v["quotaPerUnit"].as_f64() {
        n
    } else {
        500000.0
    };

    // currency: defaults to "$"
    let currency: String = match v["currency"].as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => "$".to_string(),
    };

    NewApiCreds::Ok(NewApiCredsData {
        base_url,
        access_token,
        user_id,
        quota_per_unit,
        currency,
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("USAGE_DASHBOARD_HOME", dir.path());
        dir
    }

    #[test]
    #[serial]
    fn reads_claude_token_from_file() {
        let dir = setup();
        let p = dir.path().join(".claude");
        fs::create_dir_all(&p).unwrap();
        fs::write(
            p.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"abc123"}}"#,
        )
        .unwrap();
        assert_eq!(claude_token().as_deref(), Some("abc123"));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    #[serial]
    fn codex_missing_file() {
        let _d = setup();
        assert!(matches!(codex_creds(), CodexCreds::MissingFile));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    #[serial]
    fn new_api_rejects_traversal() {
        let _d = setup();
        assert!(matches!(new_api_creds("../evil.json"), NewApiCreds::InvalidPath));
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }

    #[test]
    #[serial]
    fn new_api_reads_fields_with_defaults() {
        let dir = setup();
        let cfgdir = crate::paths::config_dir();
        fs::create_dir_all(&cfgdir).unwrap();
        fs::write(
            cfgdir.join("phanrouter.json"),
            r#"{"baseUrl":"https://x/new-api","accessToken":"tok","userId":"7"}"#,
        )
        .unwrap();
        match new_api_creds("phanrouter.json") {
            NewApiCreds::Ok(c) => {
                assert_eq!(c.user_id, 7);
                assert_eq!(c.quota_per_unit, 500000.0);
                assert_eq!(c.currency, "$");
            }
            _ => panic!("expected ok"),
        }
        let _ = dir;
        std::env::remove_var("USAGE_DASHBOARD_HOME");
    }
}
