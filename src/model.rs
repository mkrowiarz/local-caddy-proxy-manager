use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    NotDeployed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceSource {
    Compose { file: PathBuf, service_name: String },
    Runtime,
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub domain: String,
    pub port: u16,
    pub tls: String,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub proxy: Option<ProxyConfig>,
    pub status: ContainerStatus,
    pub source: ServiceSource,
    pub project: String,
    pub available_ports: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaddyProxyStatus {
    Up,
    Down,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum CaddyControlMethod {
    Systemd,
    Container,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Project,
    Global,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveModal {
    None,
    AddProxy,
    EditProxy,
    CaddyMenu,
    Help,
}

#[derive(Debug, Clone)]
pub struct FormState {
    pub focused_field: usize,
    pub domain: String,
    pub port: String,
    pub tls: String,
    pub service_index: usize,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            focused_field: 0,
            domain: String::new(),
            port: String::new(),
            tls: "internal".to_string(),
            service_index: 0,
        }
    }
}

// Serde structs for compose YAML parsing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeFile {
    pub name: Option<String>,
    #[serde(default)]
    pub services: HashMap<String, ComposeService>,
    #[serde(default)]
    pub networks: HashMap<String, Option<ComposeNetwork>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeService {
    #[serde(default)]
    pub labels: ComposeLabels,
    #[serde(default)]
    pub ports: Vec<serde_yaml_ng::Value>,
    #[serde(default)]
    pub expose: Vec<serde_yaml_ng::Value>,
    #[serde(default)]
    pub networks: Option<serde_yaml_ng::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum ComposeLabels {
    #[default]
    None,
    Map(HashMap<String, String>),
    List(Vec<String>),
}

impl ComposeLabels {
    pub fn to_map(&self) -> HashMap<String, String> {
        match self {
            ComposeLabels::None => HashMap::new(),
            ComposeLabels::Map(m) => m.clone(),
            ComposeLabels::List(list) => {
                let mut map = HashMap::new();
                for item in list {
                    if let Some((k, v)) = item.split_once('=') {
                        map.insert(k.to_string(), v.to_string());
                    }
                }
                map
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeNetwork {
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
