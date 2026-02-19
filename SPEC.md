# lcp — Local Caddy Proxy Manager

## Context

Managing caddy-docker-proxy services is currently manual: you write caddy labels in compose files by hand, remember the label syntax, and have no visibility into what's actually proxied across projects. `lcp` is a Rust TUI tool that provides a dashboard of all proxied services, auto-detects unproxied compose services, and streamlines adding/editing caddy labels — with a live preview pane for copy-paste workflows.

**Separate repo/crate**, installable via `cargo install lcp`.

---

## Architecture

```
lcp (binary)
├── Docker-compatible API (socket) ── podman or docker
│   ├── List containers with caddy.* labels (global view)
│   ├── Inspect container labels, networks, ports
│   └── Container lifecycle (for caddy-proxy management)
├── Caddy Admin API (localhost:2019) ── route verification
│   └── GET /config/ to verify active routes
├── Compose file parser (serde_yaml)
│   ├── Read services, labels, networks, ports
│   └── Write modified compose files (full rewrite OK)
└── TUI (ratatui + crossterm)
    ├── Project view (current directory)
    └── Global view (all proxied containers)
```

### Container Runtime

- Use the **Docker-compatible API** via Unix socket — works with both podman and docker
- Auto-detect socket: check `DOCKER_HOST` env var, then `$XDG_RUNTIME_DIR/podman/podman.sock`, then `/var/run/docker.sock`
- Rust crate: **bollard** (async Docker client, podman-compatible)

### Caddy Admin API

- Query `http://localhost:2019/config/` for active route verification
- Requires exposing port 2019 on caddy-proxy (user will update their service/compose config)
- Used to show whether a caddy-labeled container's route is actually active in Caddy
- Gracefully degrade if admin API is unreachable (show "unknown" status)

### Compose File Discovery (Project View)

Scan from current working directory:
1. Root level: `compose*.y{a,}ml`, `docker-compose*.y{a,}ml`
2. Recursive: `**/compose*.y{a,}ml`
3. **Filter out**: filenames containing `prod`, `staging`, `production`
4. Track which file each service originates from (needed for `-f` flag on compose up)

### Compose File Modification

- Full YAML rewrite is acceptable (no format preservation needed)
- Use **serde_yaml** for parse + serialize
- When adding caddy labels to a service:
  - **Smart network detection**: check if `caddy` external network is defined in the compose file; if not, add it to both the top-level `networks:` section and the service's `networks:` list
  - Add labels: `caddy`, `caddy.reverse_proxy`, `caddy.tls`
- After writing, auto-apply: run `docker compose -f <file> up -d` (or `podman compose`) with the correct `-f` flag for the originating file

---

## TUI Layout

htop-style full-width table with two tab-switchable views:

```
┌─ lcp ─────────────────────────────── caddy-proxy: ● UP ─┐
│ [Project] [Global]                              [r]efresh│
├──────────────────────────────────────────────────────────┤
│ Domain                  Port  Status   TLS       Source  │
│ ──────────────────────────────────────────────────────── │
│ > api.myapp.localhost   3000  ● run    internal  compose │
│   web.myapp.localhost   8080  ● run    internal  compose │
│   admin.myapp.localhost 5432  ○ stop   internal  compose │
│                                                          │
│ ── Available (no proxy) ─────────────────────────────── │
│   + redis               6379                     compose │
│   + worker              -                        compose │
├──────────────────────────────────────────────────────────┤
│ [a]dd [e]dit [o]pen [r]efresh [?]help  Tab: switch view  │
└──────────────────────────────────────────────────────────┘
```

### Global View

Same table layout but shows ALL proxied containers from the container runtime, grouped or annotated by project. The "Source" column shows the compose project name or "manual" for containers without a compose origin.

### No Project Context

When run outside a project directory (no compose files found), skip project view — open directly to global view.

---

## Views & Navigation

| Key | Action |
|-----|--------|
| `Tab` | Switch between Project / Global view |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `a` | Add proxy to selected unproxied service (popup form) |
| `e` | Edit proxy config of selected proxied service (popup form) |
| `o` | Open selected service URL in browser (always HTTPS) |
| `r` | Refresh — re-query container runtime + compose files |
| `c` | Caddy-proxy management submenu (start/stop/restart) |
| `?` | Help overlay |
| `q` / `Esc` | Quit |

---

## Features

### 1. Service Dashboard (Project + Global views)

**Data sources merged:**
- **Compose files** on disk: parse services, their caddy labels, networks, ports
- **Container runtime**: query running/stopped containers with caddy.* labels
- **Caddy admin API**: verify which routes are actually active

**Table columns:**
| Column | Source |
|--------|--------|
| Domain | `caddy` label value |
| Port | `caddy.reverse_proxy` label (parsed) |
| Status | Container runtime state (running/stopped/not deployed) |
| TLS | `caddy.tls` label value |
| Source | Compose filename or "runtime" for non-compose containers |

**Unproxied services** (project view only): services in compose files without caddy labels, shown in a separate section below the proxied list.

### 2. Add Proxy (popup form + preview pane)

Triggered by pressing `a` on an unproxied service.

**Form fields (pre-filled with smart defaults):**
| Field | Default | Source |
|-------|---------|--------|
| Domain | `<service>.<project>.localhost` | Service name from compose + project name (compose `name:` field, then directory name) |
| Port | First exposed/mapped port | Compose `ports:` or `expose:` section |
| TLS | `internal` | Always |

**Preview pane**: Live-updating YAML preview alongside the form, showing exactly what labels and network config will be added. Can be copied without persisting (for users who want to paste manually).

**On confirm ("Save"):**
1. Add `caddy` external network to compose file if missing (top-level + service)
2. Add caddy labels to the service
3. Write compose file (full rewrite via serde_yaml)
4. Run `docker/podman compose -f <file> up -d` to apply

### 3. Edit Proxy Config (popup form)

Same form as Add, but pre-filled with current values from existing labels. Same preview pane. Same write + apply flow on confirm.

### 4. Open in Browser

Press `o` on a proxied service → open `https://<domain>` in the default browser. Use `xdg-open` (Linux) or `open` (macOS).

### 5. Caddy-Proxy Management

Status indicator in the header bar: `caddy-proxy: ● UP` or `caddy-proxy: ○ DOWN`.

Press `c` to open a submenu:
- **Start** caddy-proxy
- **Stop** caddy-proxy
- **Restart** caddy-proxy

**Control method auto-detection:**
1. Check if `caddy-proxy.service` exists in systemd user scope → use `systemctl --user start/stop/restart caddy-proxy`
2. Otherwise → use container runtime directly: `docker/podman start/stop/restart caddy-proxy`

---

## Domain Convention

```
<service>.<project>.localhost
```

- **service**: compose service name (e.g. `api`, `web`, `db`)
- **project**: compose `name:` field if set, otherwise the directory name containing the compose file
- **`.localhost`**: RFC 6761 — resolves to 127.0.0.1 natively, including nested subdomains

Example: service `api` in project `myapp` → `api.myapp.localhost`

---

## Tech Stack

| Component | Crate | Purpose |
|-----------|-------|---------|
| TUI framework | `ratatui` + `crossterm` | Terminal rendering + input handling |
| Docker API | `bollard` | Container inspection, lifecycle, events |
| HTTP client | `reqwest` | Caddy admin API queries |
| YAML | `serde_yaml` | Compose file parse/serialize |
| Async runtime | `tokio` | Required by bollard + reqwest |
| CLI framework | `clap` | `--help`, `--version`, future subcommands |
| Browser open | `open` crate | Cross-platform `xdg-open`/`open` |
| Error handling | `anyhow` | Ergonomic error propagation |

---

## Project Structure

```
lcp/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, tokio runtime, app loop
│   ├── app.rs               # App state, view enum, event handling
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── dashboard.rs     # Main table rendering (project + global)
│   │   ├── form.rs          # Add/edit popup form
│   │   ├── preview.rs       # YAML preview pane
│   │   ├── caddy_menu.rs    # Caddy-proxy management submenu
│   │   └── help.rs          # Help overlay
│   ├── docker/
│   │   ├── mod.rs
│   │   ├── client.rs        # bollard client init, socket auto-detection
│   │   ├── containers.rs    # List/inspect containers, parse caddy labels
│   │   └── compose.rs       # Compose command execution (up -d)
│   ├── caddy/
│   │   ├── mod.rs
│   │   └── admin.rs         # Caddy admin API client (:2019)
│   ├── compose/
│   │   ├── mod.rs
│   │   ├── discovery.rs     # Find compose files in cwd (recursive, filter prod)
│   │   ├── parser.rs        # Parse compose YAML → service structs
│   │   └── writer.rs        # Modify + write compose files
│   └── model.rs             # Shared types: Service, ProxyConfig, CaddyStatus
└── README.md
```

---

## Not in v1

- Toggle proxy on/off (label immutability makes this complex)
- Log viewer (use `podman logs caddy-proxy` directly)
- CLI subcommands (TUI only)
- Config file (auto-detect everything)
- Parent directory compose file search

---

## Verification

1. **Build**: `cargo build` compiles without errors
2. **No runtime, no compose**: Run `lcp` in an empty directory → should show global view with caddy-proxy status
3. **With compose file**: Create a test compose.yml with a service, run `lcp` → project view shows the service as "unproxied"
4. **Add proxy**: Press `a`, fill form, verify preview pane shows correct YAML, confirm → compose file updated, `compose up -d` runs
5. **Global view**: Tab to global → shows the newly proxied container with correct domain/port/status
6. **Edit**: Press `e` on proxied service, change port, confirm → compose file updated, container recreated
7. **Open browser**: Press `o` → browser opens `https://service.project.localhost`
8. **Caddy management**: Press `c` → start/stop/restart work via auto-detected method
9. **Cross-runtime**: Test with both podman socket and docker socket

## Prerequisite Change

The existing caddy-proxy setup needs port 2019 exposed for the admin API. Update:
- `home/dot_config/systemd/user/caddy-proxy.service`: add `-p 2019:2019` to ExecStart
- `home/dot_local/share/caddy-proxy/compose.yml`: add `"2019:2019"` to ports
