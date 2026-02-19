use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

use crate::model::ProxyConfig;

/// Write or update a `compose.lcp.yaml` file with caddy proxy config for a service.
/// Preserves previously added services in the file.
pub fn write_lcp_file(lcp_file_path: &Path, service_name: &str, config: &ProxyConfig) -> Result<()> {
    // Read existing file if present, to preserve other services
    let mut doc: BTreeMap<String, serde_yaml_ng::Value> = if lcp_file_path.exists() {
        let content = std::fs::read_to_string(lcp_file_path)
            .with_context(|| format!("Failed to read {}", lcp_file_path.display()))?;
        serde_yaml_ng::from_str(&content).unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    // Build the service entry
    let mut labels = serde_yaml_ng::Mapping::new();
    labels.insert(
        serde_yaml_ng::Value::String("caddy".to_string()),
        serde_yaml_ng::Value::String(config.domain.clone()),
    );
    labels.insert(
        serde_yaml_ng::Value::String("caddy.reverse_proxy".to_string()),
        serde_yaml_ng::Value::String(format!("{{{{upstreams {}}}}}", config.port)),
    );
    labels.insert(
        serde_yaml_ng::Value::String("caddy.tls".to_string()),
        serde_yaml_ng::Value::String(config.tls.clone()),
    );

    let mut service_map = serde_yaml_ng::Mapping::new();
    service_map.insert(
        serde_yaml_ng::Value::String("labels".to_string()),
        serde_yaml_ng::Value::Mapping(labels),
    );
    service_map.insert(
        serde_yaml_ng::Value::String("networks".to_string()),
        serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::String("caddy".to_string())]),
    );

    // Get or create the services mapping
    let services = doc
        .entry("services".to_string())
        .or_insert_with(|| serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new()));

    if let serde_yaml_ng::Value::Mapping(ref mut m) = services {
        m.insert(
            serde_yaml_ng::Value::String(service_name.to_string()),
            serde_yaml_ng::Value::Mapping(service_map),
        );
    }

    // Add top-level networks with caddy external
    let mut caddy_net = serde_yaml_ng::Mapping::new();
    caddy_net.insert(
        serde_yaml_ng::Value::String("external".to_string()),
        serde_yaml_ng::Value::Bool(true),
    );
    let mut networks = serde_yaml_ng::Mapping::new();
    networks.insert(
        serde_yaml_ng::Value::String("caddy".to_string()),
        serde_yaml_ng::Value::Mapping(caddy_net),
    );
    doc.insert("networks".to_string(), serde_yaml_ng::Value::Mapping(networks));

    let yaml = serde_yaml_ng::to_string(&doc)
        .context("Failed to serialize compose.lcp.yaml")?;
    std::fs::write(lcp_file_path, yaml)
        .with_context(|| format!("Failed to write {}", lcp_file_path.display()))?;

    Ok(())
}

/// Generate a YAML preview showing what compose.lcp.yaml will contain for this service.
pub fn generate_preview(service_name: &str, config: &ProxyConfig) -> String {
    format!(
        r#"# compose.lcp.yaml
services:
  {}:
    labels:
      caddy: {}
      caddy.reverse_proxy: "{{{{upstreams {}}}}}"
      caddy.tls: {}
    networks:
      - caddy

networks:
  caddy:
    external: true"#,
        service_name, config.domain, config.port, config.tls
    )
}
