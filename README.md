# Proxmox ZNC LXC

A small Proxmox host-side installer that creates an Alpine LXC and bootstraps a basic ZNC bounce.

The shell installer is the current working path. A Rust CLI scaffold has now been added as the cleaner interactive direction for the next iteration.

## Defaults

- IRC network: `irc.libera.chat`
- IRC port: `6697`
- ZNC network name: `libera`
- Container bridge: `vmbr0`
- Container RAM: `256 MB`
- Container swap: `256 MB`
- Container disk: `2 GB`
- Container CPU cores: `1`

## What it does

- Downloads the latest Alpine LXC template for the host architecture.
- Creates a small unprivileged container.
- Installs `znc`, `znc-openrc`, and `ca-certificates`.
- Generates a basic ZNC config wired to Libera by default via `znc --makeconf`.
- Starts the service and enables it on boot.

## Usage

Run it on the Proxmox host as `root`:

```bash
chmod +x proxmox-znc.sh
./proxmox-znc.sh
```

Dry-run it from a fetched copy:

```bash
curl -fsSL https://raw.githubusercontent.com/rickycodes/proxmox_znc/main/proxmox-znc.sh | bash -s -- --dry-run
```

Or pass overrides:

```bash
./proxmox-znc.sh \
  --hostname znc \
  --nick ricky \
  --znc-user rickyznc \
  --irc-network libera \
  --irc-server irc.libera.chat \
  --bridge vmbr0 \
  --storage local-lvm
```

## Rust Scaffold

The repo now includes an initial Rust CLI scaffold in `src/` and `Cargo.toml`.

Planned shape:
- interactive prompts for the install values
- Proxmox container creation from Rust
- ZNC bootstrap from Rust
- a tiny `scripts/install.sh` wrapper for `curl`-based installs
- a GitHub Actions release workflow that builds and publishes Linux binaries

The Rust code is intentionally minimal right now and needs a GitHub Release before `scripts/install.sh` can be used end to end.

## Release Flow

Create a tag like `v0.1.0` and push it:

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions will build:
- `proxmox-znc-x86_64`
- `proxmox-znc-aarch64`

and attach them to the release.

Then the install wrapper can fetch the right binary from:

```bash
curl -fsSL https://raw.githubusercontent.com/rickycodes/proxmox_znc/main/scripts/install.sh | bash
```

## Install-Time Knobs

- `--hostname`: container hostname
- `--nick`: IRC nick inside the network; also used as the ZNC admin username by default
- `--alt-nick`: fallback nick
- `--znc-user`: ZNC admin username
- `--password`: ZNC password
- `--irc-server`: IRC server hostname
- `--irc-port`: IRC server port
- `--irc-network`: network name used in the ZNC login string
- `--bridge`: Proxmox bridge
- `--storage`: container root disk storage
- `--memory`, `--swap`, `--disk`, `--cores`: container sizing
- `--dry-run`: print the planned container and ZNC settings, then exit

## After Install

- IRC client login format: `<znc-user>/<network>:<password>`
- Default IRC server inside ZNC: `irc.libera.chat:6697`

The script leaves ZNC’s generated config mostly alone. If you want to tune modules, listeners, or auth later, do that in ZNC itself after install.
