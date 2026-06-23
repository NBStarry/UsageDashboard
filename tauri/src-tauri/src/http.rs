#![allow(dead_code)]

use std::time::Duration;

pub async fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<(serde_json::Value, u16), String> {
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
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
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
    async fn surfaces_non_2xx_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/u"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({})))
            .mount(&server).await;
        let url = format!("{}/u", server.uri());
        let (_v, status) = get_json(&url, &[]).await.unwrap();
        assert_eq!(status, 401);
    }
}
