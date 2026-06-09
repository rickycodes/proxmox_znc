#!/usr/bin/env bash
set -euo pipefail

SCRIPT_NAME="$(basename "$0")"

log() {
  printf '[%s] %s\n' "$SCRIPT_NAME" "$*"
}

die() {
  printf '[%s] %s\n' "$SCRIPT_NAME" "$*" >&2
  exit 1
}

usage() {
  cat <<EOF
Usage: $SCRIPT_NAME [options]

Create a small Alpine LXC on Proxmox and bootstrap a basic ZNC bounce.

Defaults:
  IRC server:       irc.libera.chat
  IRC port:         6697
  IRC network name: libera
  Container bridge: vmbr0
  Container RAM:    256 MB
  Container swap:   256 MB
  Container disk:   2 GB
  Container cores:  1

Options:
  --hostname NAME       Container hostname
  --storage NAME        Root disk storage for the container
  --template-storage N  Storage used for the Alpine template download
  --bridge NAME         Proxmox bridge to attach the container to
  --memory MB           RAM limit for the container
  --swap MB             Swap limit for the container
  --disk GB             Root disk size in GB
  --cores N             CPU cores for the container
  --znc-user NAME       ZNC admin username
  --nick NAME           IRC nick
  --alt-nick NAME       IRC alternate nick
  --realname NAME       IRC real name
  --password PASS       ZNC password
  --irc-server HOST     IRC server hostname
  --irc-port PORT       IRC server port
  --irc-network NAME    ZNC IRC network name
  --web-port PORT       ZNC listener port inside the container
  --help                Show this help

Environment variables with the same names are also honored.
EOF
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

prompt_default() {
  local var_name="$1"
  local prompt="$2"
  local default_value="$3"
  local value="${!var_name:-}"

  if [[ -n "$value" ]]; then
    printf -v "$var_name" '%s' "$value"
    return
  fi

  read -r -p "$prompt [$default_value]: " value
  if [[ -z "$value" ]]; then
    value="$default_value"
  fi
  printf -v "$var_name" '%s' "$value"
}

prompt_secret() {
  local var_name="$1"
  local prompt="$2"
  local value="${!var_name:-}"

  if [[ -n "$value" ]]; then
    printf -v "$var_name" '%s' "$value"
    return
  fi

  while true; do
    read -r -s -p "$prompt: " value
    printf '\n'
    [[ -n "$value" ]] || continue
    printf -v "$var_name" '%s' "$value"
    return
  done
}

map_arch() {
  case "$1" in
    x86_64) printf '%s' amd64 ;;
    aarch64) printf '%s' arm64 ;;
    armv7l|armv7) printf '%s' armv7 ;;
    riscv64) printf '%s' riscv64 ;;
    *)
      die "unsupported host architecture for Alpine template lookup: $1"
      ;;
  esac
}

pick_next_ctid() {
  if command -v pvesh >/dev/null 2>&1; then
    pvesh get /cluster/nextid
    return
  fi

  die "pvesh is required to auto-select a container ID; pass --ctid instead"
}

download_alpine_template() {
  local template_storage="$1"
  local template_arch="$2"
  local template_name=""

  log "refreshing Alpine template list"
  pveam update >/dev/null

  template_name="$(
    pveam available \
      | awk -v arch="$template_arch" '$2 ~ "^alpine-.*-default_.*_" arch "\\.tar\\.xz$" { print $2 }' \
      | sort -Vr \
      | head -n1
  )"

  [[ -n "$template_name" ]] || die "could not find an Alpine template for architecture $template_arch"

  if ! pveam list "$template_storage" 2>/dev/null | awk '{ print $1 }' | grep -qx "$template_name"; then
    log "downloading template $template_name to storage $template_storage"
    pveam download "$template_storage" "$template_name"
  fi

  printf '%s:vztmpl/%s' "$template_storage" "$template_name"
}

create_container() {
  local ctid="$1"
  local hostname="$2"
  local storage="$3"
  local template_ref="$4"
  local bridge="$5"
  local memory="$6"
  local swap="$7"
  local disk="$8"
  local cores="$9"

  log "creating container $ctid ($hostname)"
  pct create "$ctid" "$template_ref" \
    --hostname "$hostname" \
    --ostype alpine \
    --unprivileged 1 \
    --cores "$cores" \
    --memory "$memory" \
    --swap "$swap" \
    --rootfs "${storage}:${disk}" \
    --net0 "name=eth0,bridge=${bridge},ip=dhcp" \
    --onboot 1
}

bootstrap_container() {
  local ctid="$1"
  local bootstrap
  bootstrap="$(mktemp)"

  cat >"$bootstrap" <<'EOF'
#!/bin/sh
set -eu

apk add --no-cache ca-certificates znc znc-openrc >/dev/null

if ! id znc >/dev/null 2>&1; then
  adduser -D -h /var/lib/znc -s /sbin/nologin znc
fi

install -d -o znc -g znc /var/lib/znc
install -d -o znc -g znc /var/lib/znc/configs

answers="$(mktemp)"
trap 'rm -f "$answers"' EXIT

cat >"$answers" <<ANSWERS
$WEB_PORT
yes
no

$ZNC_USER
$ZNC_PASSWORD
$ZNC_PASSWORD
$IRC_NICK
$IRC_ALT_NICK
$ZNC_USER
$IRC_REALNAME

yes
$IRC_NETWORK
$IRC_SERVER
yes
$IRC_PORT


no
ANSWERS

chown znc:znc "$answers"

su -s /bin/sh znc -c "HOME=/var/lib/znc znc --datadir=/var/lib/znc --makeconf" <"$answers" >/tmp/znc-makeconf.log 2>&1 || {
  cat /tmp/znc-makeconf.log >&2
  exit 1
}

config="/var/lib/znc/configs/znc.conf"
if ! grep -qx 'LoadModule = webadmin' "$config"; then
  tmp_config="$(mktemp)"
  awk '
    BEGIN { inserted = 0 }
    /^<Listener / && inserted == 0 {
      print "LoadModule = webadmin"
      inserted = 1
    }
    { print }
    END {
      if (inserted == 0) {
        print "LoadModule = webadmin"
      }
    }
  ' "$config" >"$tmp_config"
  mv "$tmp_config" "$config"
fi

chown -R znc:znc /var/lib/znc
rc-update add znc default >/dev/null
rc-service znc start >/dev/null
EOF

  pct exec "$ctid" -- env \
    WEB_PORT="$WEB_PORT" \
    ZNC_USER="$ZNC_USER" \
    ZNC_PASSWORD="$ZNC_PASSWORD" \
    IRC_NICK="$IRC_NICK" \
    IRC_ALT_NICK="$IRC_ALT_NICK" \
    IRC_REALNAME="$IRC_REALNAME" \
    IRC_NETWORK="$IRC_NETWORK" \
    IRC_SERVER="$IRC_SERVER" \
    IRC_PORT="$IRC_PORT" \
    /bin/sh -s <"$bootstrap"

  rm -f "$bootstrap"
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        usage
        exit 0
        ;;
      --hostname)
        CT_HOSTNAME="${2:-}"; shift 2 ;;
      --storage)
        STORAGE="${2:-}"; shift 2 ;;
      --template-storage)
        TEMPLATE_STORAGE="${2:-}"; shift 2 ;;
      --bridge)
        BRIDGE="${2:-}"; shift 2 ;;
      --memory)
        MEMORY="${2:-}"; shift 2 ;;
      --swap)
        SWAP="${2:-}"; shift 2 ;;
      --disk)
        DISK="${2:-}"; shift 2 ;;
      --cores)
        CORES="${2:-}"; shift 2 ;;
      --znc-user)
        ZNC_USER="${2:-}"; shift 2 ;;
      --nick)
        NICK="${2:-}"; shift 2 ;;
      --alt-nick)
        ALT_NICK="${2:-}"; shift 2 ;;
      --realname)
        REALNAME="${2:-}"; shift 2 ;;
      --password)
        PASSWORD="${2:-}"; shift 2 ;;
      --irc-server)
        IRC_SERVER="${2:-}"; shift 2 ;;
      --irc-port)
        IRC_PORT="${2:-}"; shift 2 ;;
      --irc-network)
        IRC_NETWORK="${2:-}"; shift 2 ;;
      --web-port)
        WEB_PORT="${2:-}"; shift 2 ;;
      --)
        shift
        break
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
  done

  require_cmd pct
  require_cmd pveam
  require_cmd awk
  require_cmd sort
  require_cmd grep
  require_cmd mktemp

  [[ $EUID -eq 0 ]] || die "run this on the Proxmox host as root"

  local ctid
  local hostname="${CT_HOSTNAME:-znc}"
  local storage="${STORAGE:-local-lvm}"
  local template_storage="${TEMPLATE_STORAGE:-local}"
  local bridge="${BRIDGE:-vmbr0}"
  local memory="${MEMORY:-256}"
  local swap="${SWAP:-256}"
  local disk="${DISK:-2}"
  local cores="${CORES:-1}"
  local znc_user="${ZNC_USER:-znc}"
  local irc_nick="${NICK:-}"
  local irc_alt_nick="${ALT_NICK:-}"
  local irc_realname="${REALNAME:-}"
  local znc_password="${PASSWORD:-}"
  local irc_server="${IRC_SERVER:-irc.libera.chat}"
  local irc_port="${IRC_PORT:-6697}"
  local irc_network="${IRC_NETWORK:-libera}"
  local web_port="${WEB_PORT:-6697}"

  ctid="$(pick_next_ctid)"

  prompt_default hostname "Container hostname" "$hostname"
  prompt_default storage "Root disk storage" "$storage"
  prompt_default template_storage "Template storage" "$template_storage"
  prompt_default bridge "Proxmox bridge" "$bridge"
  prompt_default memory "Container RAM (MB)" "$memory"
  prompt_default swap "Container swap (MB)" "$swap"
  prompt_default disk "Container root disk (GB)" "$disk"
  prompt_default cores "Container CPU cores" "$cores"
  prompt_default znc_user "ZNC admin username" "$znc_user"
  prompt_default irc_nick "IRC nick" "${irc_nick:-$znc_user}"
  prompt_default irc_alt_nick "IRC alternate nick" "${irc_alt_nick:-${irc_nick}_}"
  prompt_default irc_realname "IRC real name" "${irc_realname:-$irc_nick}"
  prompt_default irc_server "IRC server" "$irc_server"
  prompt_default irc_port "IRC server port" "$irc_port"
  prompt_default irc_network "IRC network name" "$irc_network"
  prompt_default web_port "ZNC listener port" "$web_port"
  prompt_secret znc_password "ZNC password"

  local arch template_arch template_ref
  arch="$(uname -m)"
  template_arch="$(map_arch "$arch")"
  template_ref="$(download_alpine_template "$template_storage" "$template_arch")"

  create_container "$ctid" "$hostname" "$storage" "$template_ref" "$bridge" "$memory" "$swap" "$disk" "$cores"
  pct start "$ctid"
  bootstrap_container "$ctid"

  local container_ip
  container_ip="$(pct exec "$ctid" -- hostname -I 2>/dev/null | awk '{ print $1 }' || true)"

  log "done"
  printf '\n'
  printf 'Container ID: %s\n' "$ctid"
  printf 'Hostname: %s\n' "$hostname"
  if [[ -n "$container_ip" ]]; then
    printf 'Container IP: %s\n' "$container_ip"
    printf 'ZNC webadmin: https://%s:%s/\n' "$container_ip" "$web_port"
    printf 'IRC client login format: %s/%s:<password>\n' "$znc_user" "$irc_network"
  fi
  printf 'IRC server inside ZNC: %s:%s\n' "$irc_server" "$irc_port"
  printf 'ZNC user: %s\n' "$znc_user"
  printf 'IRC nick: %s\n' "$irc_nick"
}

main "$@"
