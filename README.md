# Proxmox ZNC LXC

A small Proxmox host-side installer that creates an Alpine LXC and bootstraps a basic ZNC bounce.

## Defaults

- IRC network: `irc.libera.chat`
- IRC port: `6697`
- ZNC IRC listener port: `6697`
- ZNC webadmin port: `6698`
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
- Generates a basic ZNC config wired to Libera by default.
- Loads `watch`, `chansaver`, and `controlpanel` by default.
- Starts the service and enables it on boot.

## Usage

Run it on the Proxmox host as `root`:

```bash
chmod +x proxmox-znc.sh
./proxmox-znc.sh
```

Dry-run it from a fetched copy:

```bash
curl -fsSL https://raw.githubusercontent.com/<you>/<repo>/main/proxmox-znc.sh | bash -s -- --dry-run
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

## Install-Time Knobs

- `--hostname`: container hostname
- `--nick`: IRC nick inside the network
- `--alt-nick`: fallback nick
- `--znc-user`: ZNC admin username
- `--password`: ZNC password
- `--irc-server`: IRC server hostname
- `--irc-port`: IRC server port
- `--irc-network`: network name used in the ZNC login string
- `--listener-port`: IRC client listener port inside the container
- `--web-port`: web admin listener port inside the container
- `--auth-mode`: choose `none`, `sasl`, or `nickserv`
- `--bridge`: Proxmox bridge
- `--storage`: container root disk storage
- `--memory`, `--swap`, `--disk`, `--cores`: container sizing
- `--dry-run`: print the planned container and ZNC settings, then exit

## After Install

- IRC listener: `ssl://<container-ip>:6697`
- Web admin: `https://<container-ip>:6698/`
- IRC client login format: `<znc-user>/<network>:<password>`
- Default IRC server inside ZNC: `irc.libera.chat:6697`
- Default user modules: `watch`, `chansaver`, `controlpanel`

If you want to change channels, modules, or buffers later, use the ZNC web admin UI.
