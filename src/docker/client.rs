use anyhow::{Context, Result};
use bollard::Docker;

#[derive(Debug, Clone)]
pub enum RuntimeType {
    Docker,
    Podman,
}

pub struct DockerClient {
    pub docker: Docker,
    pub runtime: RuntimeType,
    pub socket_path: String,
}

/// Auto-detect Docker/Podman socket and connect via bollard.
/// Priority: $DOCKER_HOST env var → podman socket → docker socket
pub async fn connect() -> Result<DockerClient> {
    // 1. Try $DOCKER_HOST env var (bollard handles this internally)
    if std::env::var("DOCKER_HOST").is_ok() {
        if let Ok(docker) = Docker::connect_with_defaults() {
            return Ok(DockerClient {
                docker,
                runtime: RuntimeType::Docker,
                socket_path: std::env::var("DOCKER_HOST").unwrap_or_default(),
            });
        }
    }

    // 2. Try podman socket at $XDG_RUNTIME_DIR/podman/podman.sock
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        // Fallback: try to get UID via `id -u`
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| format!("/run/user/{}", s.trim()))
            .unwrap_or_else(|| "/run/user/1000".to_string())
    });
    let podman_sock = format!("{}/podman/podman.sock", xdg_runtime);
    if std::path::Path::new(&podman_sock).exists() {
        if let Ok(docker) = Docker::connect_with_unix(&podman_sock, 120, bollard::API_DEFAULT_VERSION) {
            // Verify it's actually reachable
            if docker.ping().await.is_ok() {
                return Ok(DockerClient {
                    docker,
                    runtime: RuntimeType::Podman,
                    socket_path: podman_sock,
                });
            }
        }
    }

    // 3. Try default docker socket
    let docker_sock = "/var/run/docker.sock";
    if std::path::Path::new(docker_sock).exists() {
        let docker = Docker::connect_with_unix(docker_sock, 120, bollard::API_DEFAULT_VERSION)
            .context("Failed to connect to Docker socket")?;
        if docker.ping().await.is_ok() {
            return Ok(DockerClient {
                docker,
                runtime: RuntimeType::Docker,
                socket_path: docker_sock.to_string(),
            });
        }
    }

    // 4. Fall back to bollard defaults (may use DOCKER_HOST or default socket)
    let docker = Docker::connect_with_defaults()
        .context("No Docker/Podman socket found. Is Docker or Podman running?")?;

    Ok(DockerClient {
        docker,
        runtime: RuntimeType::Docker,
        socket_path: docker_sock.to_string(),
    })
}

/// Return the compose command prefix ("docker" or "podman")
pub fn compose_command(runtime: &RuntimeType) -> &'static str {
    match runtime {
        RuntimeType::Docker => "docker",
        RuntimeType::Podman => "podman",
    }
}
