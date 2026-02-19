use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

use crate::model::{ComposeFile, ContainerStatus, ProxyConfig, Service, ServiceSource};

/// Name of the LCP override file written alongside user compose files.
pub const LCP_FILENAME: &str = "compose.lcp.yaml";

/// Parse a compose YAML file into a ComposeFile struct.
pub fn parse_compose_file(path: &Path) -> Result<ComposeFile> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let compose: ComposeFile = serde_yaml_ng::from_str(&content)
        .with_context(|| format!("Failed to parse YAML in {}", path.display()))?;
    Ok(compose)
}

/// Extract Service structs from a parsed ComposeFile.
/// Returns (project_name, services).
pub fn extract_services(
    compose: &ComposeFile,
    file_path: &Path,
) -> Result<(String, Vec<Service>)> {
    let project_name = compose
        .name
        .clone()
        .unwrap_or_else(|| {
            file_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    let mut services = Vec::new();

    for (name, svc) in &compose.services {
        let labels = svc.labels.to_map();
        let proxy = parse_caddy_labels(&labels);
        let available_ports = parse_ports(svc);

        services.push(Service {
            name: name.clone(),
            proxy,
            status: ContainerStatus::NotDeployed,
            source: ServiceSource::Compose {
                file: file_path.to_path_buf(),
                service_name: name.clone(),
            },
            project: project_name.clone(),
            available_ports,
        });
    }

    Ok((project_name, services))
}

/// Generate a default domain for a service: `<service>.<project>.localhost`
pub fn default_domain(service_name: &str, project_name: &str) -> String {
    format!("{}.{}.localhost", service_name, project_name)
}

/// Parse port mappings from compose service ports/expose fields.
pub fn parse_ports(service: &crate::model::ComposeService) -> Vec<u16> {
    let mut ports = HashSet::new();

    for val in &service.ports {
        if let Some(port) = extract_container_port(val) {
            ports.insert(port);
        }
    }

    for val in &service.expose {
        if let Some(port) = extract_container_port(val) {
            ports.insert(port);
        }
    }

    let mut result: Vec<u16> = ports.into_iter().collect();
    result.sort();
    result
}

/// Extract the container port from a serde_yaml_ng::Value.
/// Handles formats like "3000:3000", "3000", "0.0.0.0:3000:3000", integer values,
/// and mapping forms with `target` key.
fn extract_container_port(val: &serde_yaml_ng::Value) -> Option<u16> {
    match val {
        serde_yaml_ng::Value::Number(n) => n.as_u64().and_then(|v| u16::try_from(v).ok()),
        serde_yaml_ng::Value::String(s) => {
            // Remove protocol suffix like "/tcp", "/udp"
            let s = s.split('/').next().unwrap_or(s);
            // Formats: "3000", "3000:3000", "0.0.0.0:3000:3000", "8080:3000"
            // The container port is the last number after the last colon
            let parts: Vec<&str> = s.split(':').collect();
            let container_part = parts.last()?;
            // Handle range like "3000-3001" — take the first port
            let port_str = container_part.split('-').next()?;
            port_str.trim().parse::<u16>().ok()
        }
        serde_yaml_ng::Value::Mapping(m) => {
            // Long form: { target: 3000, published: 3000, ... }
            let target = m.get(serde_yaml_ng::Value::String("target".to_string()))?;
            extract_container_port(target)
        }
        _ => None,
    }
}

/// Parse caddy labels from a label map into a ProxyConfig.
fn parse_caddy_labels(
    labels: &std::collections::HashMap<String, String>,
) -> Option<ProxyConfig> {
    let domain = labels.get("caddy")?.clone();

    let reverse_proxy = labels.get("caddy.reverse_proxy")?;

    // Parse port from reverse_proxy value.
    // Formats: "{{upstreams 3000}}", "{{upstreams}}", "localhost:3000", ":3000"
    let port = parse_port_from_reverse_proxy(reverse_proxy)?;

    let tls = labels
        .get("caddy.tls")
        .cloned()
        .unwrap_or_else(|| "internal".to_string());

    Some(ProxyConfig { domain, port, tls })
}

/// Extract port number from a reverse_proxy label value.
fn parse_port_from_reverse_proxy(value: &str) -> Option<u16> {
    let trimmed = value.trim();

    // Try "{{upstreams PORT}}" pattern
    if trimmed.contains("upstreams") {
        // Extract digits from the value
        let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse::<u16>().ok();
        }
        return None;
    }

    // Try "host:port" or ":port" pattern
    if let Some(port_str) = trimmed.rsplit(':').next() {
        return port_str.trim().parse::<u16>().ok();
    }

    trimmed.parse::<u16>().ok()
}

/// Merge proxy configs from `compose.lcp.yaml` files into already-discovered services.
/// For each compose file directory, checks for a sibling `compose.lcp.yaml` and parses
/// caddy labels from it, updating matching services.
pub fn merge_lcp_configs(services: &mut [Service], compose_files: &[std::path::PathBuf]) {
    // Collect unique directories from compose files
    let mut dirs_seen = std::collections::HashSet::new();
    for file in compose_files {
        if let Some(dir) = file.parent() {
            if !dirs_seen.insert(dir.to_path_buf()) {
                continue;
            }
            let lcp_path = dir.join(LCP_FILENAME);
            if let Ok(lcp_compose) = parse_compose_file(&lcp_path) {
                for (svc_name, svc) in &lcp_compose.services {
                    let labels = svc.labels.to_map();
                    if let Some(proxy) = parse_caddy_labels(&labels) {
                        // Find matching service and set its proxy config
                        for service in services.iter_mut() {
                            if service.name == *svc_name && service.proxy.is_none() {
                                service.proxy = Some(proxy.clone());
                            }
                        }
                    }
                }
            }
        }
    }
}
