# lcp — Local Caddy Proxy Manager

A terminal dashboard for managing [caddy-docker-proxy](https://github.com/lucaslorentz/caddy-docker-proxy) services. Discover compose services, add/edit caddy labels, and manage caddy-proxy — all from a single TUI.

## Prerequisites

- Rust (install via [rustup](https://rustup.rs))
- Docker or Podman running
- caddy-docker-proxy set up (optional — lcp degrades gracefully without it)

For the caddy admin API status indicator, expose port 2019 on your caddy-proxy container:
```yaml
ports:
  - "2019:2019"
```

## Install

```sh
git clone git@github.com:mkrowiarz/local-caddy-proxy-manager.git
cd local-caddy-proxy-manager
cargo install --path .
```

Or directly:
```sh
cargo install --git git@github.com:mkrowiarz/local-caddy-proxy-manager.git
```

## Usage

Run from your project directory (where your `compose.yml` lives):
```sh
lcp
```

Or anywhere for the global view of all proxied containers:
```sh
lcp
```

## Keys

| Key | Action |
|-----|--------|
| `Tab` | Switch Project / Global view |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `G` | Jump to top / bottom |
| `a` | Add proxy to selected unproxied service |
| `e` | Edit proxy config of selected service |
| `o` | Open service URL in browser (`https://`) |
| `r` | Refresh |
| `c` | Caddy-proxy management (start/stop/restart) |
| `?` | Help |
| `q` / `Esc` | Quit |

## How it works

**Project view** — scans the current directory for compose files (`compose.yml`, `docker-compose.yml`, and recursive variants), shows all services. Proxied services appear at the top; unproxied services appear below with a `+` prefix.

**Global view** — queries the container runtime for all running containers with `caddy.*` labels.

**Add proxy** (`a`) — opens a form pre-filled with smart defaults (`<service>.<project>.localhost`, first exposed port, `internal` TLS). A live YAML preview shows exactly what will be written. On confirm, lcp writes the caddy labels to the compose file and runs `compose up -d`.

**Caddy label format** written by lcp:
```yaml
labels:
  caddy: api.myapp.localhost
  caddy.reverse_proxy: "{{upstreams 3000}}"
  caddy.tls: internal
networks:
  - caddy
```

## CachyOS / Podman

lcp auto-detects the container runtime. With Podman, it checks `$XDG_RUNTIME_DIR/podman/podman.sock` before falling back to `/var/run/docker.sock`. No configuration needed.
