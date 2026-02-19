use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::docker::client::{compose_command, RuntimeType};

/// Run `docker/podman compose -f <file> up -d` to apply changes.
pub async fn compose_up(file: &Path, runtime: &RuntimeType) -> Result<()> {
    let cmd = compose_command(runtime);
    let file_str = file
        .to_str()
        .context("Compose file path is not valid UTF-8")?;

    let output = tokio::process::Command::new(cmd)
        .args(["compose", "-f", file_str, "up", "-d"])
        .output()
        .await
        .with_context(|| format!("Failed to run `{} compose up`", cmd))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`{} compose up -d` failed: {}", cmd, stderr);
    }

    Ok(())
}
