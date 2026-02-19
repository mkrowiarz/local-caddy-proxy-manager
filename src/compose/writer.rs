use anyhow::{Context, Result};
use std::path::Path;

use crate::model::{ComposeFile, ComposeLabels, ComposeNetwork, ComposeService, ProxyConfig};

/// Add or update caddy labels on a service in a compose file.
/// Also ensures the caddy external network is present.
pub fn add_caddy_labels(
    compose: &mut ComposeFile,
    service_name: &str,
    config: &ProxyConfig,
) -> Result<()> {
    // Get or create the service
    let service = compose
        .services
        .entry(service_name.to_string())
        .or_default();

    // Convert existing labels to a map and add caddy labels
    let mut map = service.labels.to_map();
    map.insert("caddy".to_string(), config.domain.clone());
    map.insert(
        "caddy.reverse_proxy".to_string(),
        format!("{{{{upstreams {}}}}}", config.port),
    );
    map.insert("caddy.tls".to_string(), config.tls.clone());
    service.labels = ComposeLabels::Map(map);

    // Add "caddy" to the service's networks
    add_caddy_network_to_service(service);

    // Add "caddy" to top-level networks as external
    compose.networks.insert(
        "caddy".to_string(),
        Some(ComposeNetwork {
            external: Some(true),
            name: None,
        }),
    );

    Ok(())
}

/// Add "caddy" to a service's networks field.
fn add_caddy_network_to_service(service: &mut ComposeService) {
    let caddy_str = serde_yaml_ng::Value::String("caddy".to_string());

    match &service.networks {
        None => {
            service.networks = Some(serde_yaml_ng::Value::Sequence(vec![caddy_str]));
        }
        Some(serde_yaml_ng::Value::Sequence(seq)) => {
            let has_caddy = seq.iter().any(|v| {
                matches!(v, serde_yaml_ng::Value::String(s) if s == "caddy")
            });
            if !has_caddy {
                let mut new_seq = seq.clone();
                new_seq.push(caddy_str);
                service.networks = Some(serde_yaml_ng::Value::Sequence(new_seq));
            }
        }
        Some(serde_yaml_ng::Value::Mapping(mapping)) => {
            let key = serde_yaml_ng::Value::String("caddy".to_string());
            if !mapping.contains_key(&key) {
                let mut new_mapping = mapping.clone();
                new_mapping.insert(key, serde_yaml_ng::Value::Null);
                service.networks = Some(serde_yaml_ng::Value::Mapping(new_mapping));
            }
        }
        Some(_) => {
            // Unexpected type; replace with a list containing caddy
            service.networks = Some(serde_yaml_ng::Value::Sequence(vec![caddy_str]));
        }
    }
}

/// Write a ComposeFile to disk as YAML.
pub fn write_compose_file(compose: &ComposeFile, path: &Path) -> Result<()> {
    let yaml = serde_yaml_ng::to_string(compose)
        .context("Failed to serialize compose file to YAML")?;
    std::fs::write(path, yaml)
        .with_context(|| format!("Failed to write compose file to {}", path.display()))?;
    Ok(())
}

/// Generate a YAML preview string showing what will be added to the compose file.
pub fn generate_preview(service_name: &str, config: &ProxyConfig) -> String {
    format!(
        r#"# Labels to add to service '{}':
labels:
  caddy: {}
  caddy.reverse_proxy: "{{{{upstreams {}}}}}"
  caddy.tls: {}

# Network to add (top-level):
networks:
  caddy:
    external: true

# Network reference to add to service:
networks:
  - caddy"#,
        service_name, config.domain, config.port, config.tls
    )
}
