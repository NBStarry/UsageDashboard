use std::sync::RwLock;
use std::time::Duration;

// 运行期可配置的 HTTP 代理(来自 config.proxyUrl)。每次构建 client 时读取,
// 故运行时改代理下次取数即生效。None 表示直连。
static PROXY: RwLock<Option<String>> = RwLock::new(None);

pub fn set_proxy(url: Option<String>) {
    let cleaned = url.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    });
    if let Ok(mut g) = PROXY.write() {
        *g = cleaned;
    }
}

pub async fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<(serde_json::Value, u16), String> {
    let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(12));
    // 配了代理就让 reqwest 走它(all=http+https,经代理 CONNECT 隧道)。
    // 代理串非法时忽略(直连),不阻断取数。
    if let Some(p) = PROXY.read().ok().and_then(|g| g.clone()) {
        if let Ok(proxy) = reqwest::Proxy::all(&p) {
            builder = builder.proxy(proxy);
        }
    }
    let client = builder
        .build()
        .map_err(|e| format!("客户端初始化失败:{e}"))?;
    let mut req = client.get(url);
    for (k, v) in headers { req = req.header(*k, *v); }
    let resp = req.send().await.map_err(|e| format!("网络错误:{e}"))?;
    let status = resp.status().as_u16();
    let bytes = resp.bytes().await.map_err(|e| format!("网络错误:{e}"))?;
    let value = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| "接口返回无法解析".to_string())?;
    Ok((value, status))
}

// 直连(忽略全局代理)版 get_json。用于 relay 等私网/局域网(Tailscale)请求——不该走 GFW 代理。
pub async fn get_json_direct(url: &str, headers: &[(&str, &str)]) -> Result<(serde_json::Value, u16), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("客户端初始化失败:{e}"))?;
    let mut req = client.get(url);
    for (k, v) in headers { req = req.header(*k, *v); }
    let resp = req.send().await.map_err(|e| format!("网络错误:{e}"))?;
    let status = resp.status().as_u16();
    let bytes = resp.bytes().await.map_err(|e| format!("网络错误:{e}"))?;
    let value = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|_| "接口返回无法解析".to_string())?;
    Ok((value, status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
    #[serial]
    async fn returns_json_and_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/u")).and(header("x-test", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server).await;
        let url = format!("{}/u", server.uri());
        let (v, status) = get_json(&url, &[("x-test", "1")]).await.unwrap();
        assert_eq!(status, 200);
        assert_eq!(v["ok"], serde_json::json!(true));
    }

    #[tokio::test]
    #[serial]
    async fn surfaces_non_2xx_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/u"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({})))
            .mount(&server).await;
        let url = format!("{}/u", server.uri());
        let (_v, status) = get_json(&url, &[]).await.unwrap();
        assert_eq!(status, 401);
    }

    #[tokio::test]
    #[serial]
    async fn get_json_direct_ignores_proxy() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/u"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server).await;
        // 设一个指向黑洞的代理:若 get_json_direct 误走代理会连不上
        set_proxy(Some("http://127.0.0.1:9".to_string()));
        let (v, status) = get_json_direct(&format!("{}/u", server.uri()), &[]).await.unwrap();
        assert_eq!(status, 200);
        assert_eq!(v["ok"], serde_json::json!(true));
        set_proxy(None);
    }
}
