use anyhow::Result;
use bollard::models::ContainerSummaryStateEnum;
use bollard::Docker;
use std::collections::HashMap;

use crate::docker::client::RuntimeType;
use crate::model::{CaddyControlMethod, CaddyProxyStatus, ContainerStatus, ProxyConfig, Service, ServiceSource};

fn list_all_opts() -> bollard::query_parameters::ListContainersOptions {
    bollard::query_parameters::ListContainersOptionsBuilder::default()
        .all(true)
        .build()
}

/// List all containers with caddy.* labels, returning them as Services.
pub async fn list_caddy_services(docker: &Docker) -> Result<Vec<Service>> {
    let containers = docker.list_containers(Some(list_all_opts())).await?;
    let mut services = Vec::new();

    for container in containers {
        let labels = container.labels.unwrap_or_default();

        // Only include containers with at least one caddy label
        let has_caddy_label = labels.keys().any(|k| k == "caddy" || k.starts_with("caddy."));
        if !has_caddy_label {
            continue;
        }

        let proxy = parse_caddy_labels(&labels);
        let name = container
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let status = state_to_container_status(container.state.as_ref());

        let project = labels
            .get("com.docker.compose.project")
            .cloned()
            .unwrap_or_else(|| "runtime".to_string());

        let available_ports = container
            .ports
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.private_port)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        services.push(Service {
            name,
            proxy,
            status,
            source: ServiceSource::Runtime,
            project,
            available_ports,
        });
    }

    Ok(services)
}

/// Get current caddy-proxy container status.
pub async fn get_caddy_proxy_status(docker: &Docker) -> Result<CaddyProxyStatus> {
    let containers = docker.list_containers(Some(list_all_opts())).await?;

    for container in containers {
        let names = container.names.clone().unwrap_or_default();
        let labels = container.labels.clone().unwrap_or_default();

        let is_caddy_proxy = names.iter().any(|n| {
            let n = n.trim_start_matches('/');
            n == "caddy-proxy" || n.ends_with("_caddy-proxy") || n.ends_with("-caddy-proxy")
        }) || labels
            .get("com.docker.compose.service")
            .map(|s| s == "caddy-proxy")
            .unwrap_or(false);

        if is_caddy_proxy {
            return Ok(match container.state.as_ref() {
                Some(ContainerSummaryStateEnum::RUNNING) => CaddyProxyStatus::Up,
                _ => CaddyProxyStatus::Down,
            });
        }
    }

    Ok(CaddyProxyStatus::Unknown)
}

/// Detect whether caddy-proxy is controlled via systemd or container runtime.
pub fn detect_caddy_control_method() -> CaddyControlMethod {
    let output = std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "caddy-proxy"])
        .output();

    match output {
        Ok(o) if o.status.success() => CaddyControlMethod::Systemd,
        _ => CaddyControlMethod::Container,
    }
}

/// Start caddy-proxy using the detected control method.
pub async fn start_caddy(docker: &Docker, method: &CaddyControlMethod, runtime: &RuntimeType) -> Result<()> {
    manage_caddy(docker, method, runtime, "start").await
}

/// Stop caddy-proxy using the detected control method.
pub async fn stop_caddy(docker: &Docker, method: &CaddyControlMethod, runtime: &RuntimeType) -> Result<()> {
    manage_caddy(docker, method, runtime, "stop").await
}

/// Restart caddy-proxy using the detected control method.
pub async fn restart_caddy(docker: &Docker, method: &CaddyControlMethod, runtime: &RuntimeType) -> Result<()> {
    manage_caddy(docker, method, runtime, "restart").await
}

async fn manage_caddy(
    docker: &Docker,
    method: &CaddyControlMethod,
    runtime: &RuntimeType,
    action: &str,
) -> Result<()> {
    match method {
        CaddyControlMethod::Systemd => {
            tokio::process::Command::new("systemctl")
                .args(["--user", action, "caddy-proxy"])
                .status()
                .await?;
        }
        CaddyControlMethod::Container => {
            let containers = docker.list_containers(Some(list_all_opts())).await?;
            for container in containers {
                let names = container.names.unwrap_or_default();
                let is_caddy = names.iter().any(|n| {
                    let n = n.trim_start_matches('/');
                    n == "caddy-proxy"
                        || n.ends_with("_caddy-proxy")
                        || n.ends_with("-caddy-proxy")
                });
                if is_caddy {
                    if let Some(id) = container.id {
                        let cmd = crate::docker::client::compose_command(runtime);
                        tokio::process::Command::new(cmd)
                            .args([action, &id])
                            .status()
                            .await?;
                    }
                    break;
                }
            }
        }
    }
    Ok(())
}

/// Merge runtime container status into compose-derived services.
pub async fn merge_runtime_status(docker: &Docker, services: &mut [Service]) -> Result<()> {
    let containers = docker.list_containers(Some(list_all_opts())).await?;

    // Build a lookup: name/service-label → ContainerStatus
    let mut name_to_status: HashMap<String, ContainerStatus> = HashMap::new();
    for container in &containers {
        let cs = state_to_container_status(container.state.as_ref());
        if let Some(ref names) = container.names {
            for name in names {
                let clean = name.trim_start_matches('/').to_lowercase();
                name_to_status.insert(clean, cs.clone());
            }
        }
        if let Some(ref labels) = container.labels {
            if let Some(svc_name) = labels.get("com.docker.compose.service") {
                name_to_status.insert(svc_name.to_lowercase(), cs.clone());
            }
        }
    }

    for service in services.iter_mut() {
        let key = service.name.to_lowercase();
        if let Some(status) = name_to_status.get(&key) {
            service.status = status.clone();
        }
    }

    Ok(())
}

/// Parse caddy labels from a label map into a ProxyConfig.
pub fn parse_caddy_labels(labels: &HashMap<String, String>) -> Option<ProxyConfig> {
    let domain = labels.get("caddy")?.clone();
    let reverse_proxy = labels.get("caddy.reverse_proxy")?;
    let port = parse_port_from_reverse_proxy(reverse_proxy)?;
    let tls = labels
        .get("caddy.tls")
        .cloned()
        .unwrap_or_else(|| "internal".to_string());

    Some(ProxyConfig { domain, port, tls })
}

fn parse_port_from_reverse_proxy(value: &str) -> Option<u16> {
    let trimmed = value.trim();

    if trimmed.contains("upstreams") {
        let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse::<u16>().ok();
        }
        return None;
    }

    if let Some(port_str) = trimmed.rsplit(':').next() {
        return port_str.trim().parse::<u16>().ok();
    }

    trimmed.parse::<u16>().ok()
}

fn state_to_container_status(state: Option<&ContainerSummaryStateEnum>) -> ContainerStatus {
    match state {
        Some(ContainerSummaryStateEnum::RUNNING) => ContainerStatus::Running,
        Some(ContainerSummaryStateEnum::EXITED) | Some(ContainerSummaryStateEnum::CREATED) => {
            ContainerStatus::Stopped
        }
        None | Some(ContainerSummaryStateEnum::EMPTY) => ContainerStatus::NotDeployed,
        _ => ContainerStatus::Stopped,
    }
}
