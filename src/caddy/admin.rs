use anyhow::Result;
use std::time::Duration;

const CADDY_ADMIN_URL: &str = "http://localhost:2019";

/// Query the Caddy admin API and return active domain names.
/// Returns empty vec if admin API is unreachable (graceful degradation).
pub async fn get_active_domains() -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;

    let resp = match client
        .get(format!("{}/config/apps/http/servers", CADDY_ADMIN_URL))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Ok(vec![]),
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Ok(vec![]),
    };

    let mut domains = Vec::new();
    extract_hosts(&body, &mut domains);
    domains.sort();
    domains.dedup();
    Ok(domains)
}

/// Check if the Caddy admin API is reachable at localhost:2019.
pub async fn is_reachable() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    client
        .get(format!("{}/config/", CADDY_ADMIN_URL))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
}

/// Recursively extract hostnames from "host" arrays in match blocks.
fn extract_hosts(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(arr)) = map.get("host") {
                for h in arr {
                    if let serde_json::Value::String(s) = h {
                        out.push(s.clone());
                    }
                }
            }
            for v in map.values() {
                extract_hosts(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                extract_hosts(v, out);
            }
        }
        _ => {}
    }
}
