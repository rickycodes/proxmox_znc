#!/usr/bin/env bash
set -euo pipefail

SCRIPT_NAME="$(basename "$0")"

log() {
  printf '[%s] %s\n' "$SCRIPT_NAME" "$*" >&2
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
  --dry-run             Show what would be done and exit
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
  --help                Show this help

Environment variables with the same names are also honored.
EOF
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

prompt_fd=0
if [[ -r /dev/tty && -w /dev/tty ]]; then
  exec 3</dev/tty 4>/dev/tty
  prompt_fd=3
fi

prompt_default() {
  local var_name="$1"
  local prompt="$2"
  local default_value="$3"
  local value="${!var_name:-}"

  if [[ -n "$value" ]]; then
    printf -v "$var_name" '%s' "$value"
    return
  fi

  if [[ "$prompt_fd" -ne 0 ]]; then
    read -r -u "$prompt_fd" -p "$prompt [$default_value]: " value
  else
    read -r -p "$prompt [$default_value]: " value
  fi
  if [[ -z "$value" ]]; then
    value="$default_value"
  fi
  printf -v "$var_name" '%s' "$value"
}

run_or_echo() {
  if [[ "${DRY_RUN:-0}" -eq 1 ]]; then
    printf '+'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi

  "$@"
}

prompt_secret() {
  local var_name="$1"
  local prompt="$2"
  local value="${!var_name:-}"

  if [[ -n "$value" ]]; then
    printf -v "$var_name" '%s' "$value"
    return
  fi

  if [[ "${DRY_RUN:-0}" -eq 1 ]]; then
    printf -v "$var_name" '%s' '<not-set>'
    return
  fi

  [[ "$prompt_fd" -ne 0 ]] || die "password required but no interactive terminal is available; set PASSWORD or avoid --dry-run"

  while true; do
    read -r -u "$prompt_fd" -s -p "$prompt: " value
    if [[ "$prompt_fd" -ne 0 ]]; then
      printf '\n' >&4
    else
      printf '\n'
    fi
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

detect_nameservers() {
  local servers=()
  local line ns

  if [[ -r /etc/resolv.conf ]]; then
    while read -r line; do
      case "$line" in
        nameserver[[:space:]]*)
          ns="${line#nameserver }"
          ns="${ns%% *}"
          [[ -n "$ns" ]] || continue
          case "$ns" in
            127.*|::1)
              continue
              ;;
          esac
          servers+=("$ns")
          ;;
      esac
    done </etc/resolv.conf
  fi

  if [[ "${#servers[@]}" -eq 0 ]]; then
    servers=(1.1.1.1 8.8.8.8)
  fi

  printf '%s' "${servers[*]}"
}

download_alpine_template() {
  local template_storage="$1"
  local template_arch="$2"
  local template_name=""

  log "refreshing Alpine template list"
  pveam update >/dev/null 2>&1

  template_name="$(
    pveam available \
      | awk -v arch="$template_arch" '$2 ~ "^alpine-.*-default_.*_" arch "\\.tar\\.xz$" { print $2 }' \
      | sort -Vr \
      | head -n1
  )"

  [[ -n "$template_name" ]] || die "could not find an Alpine template for architecture $template_arch"

  if ! pveam list "$template_storage" 2>/dev/null | awk '{ print $1 }' | grep -qx "$template_name"; then
    log "downloading template $template_name to storage $template_storage"
    pveam download "$template_storage" "$template_name" >/dev/null 2>&1
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
  local nameservers="${10}"

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
    --nameserver "$nameservers" \
    --onboot 1
}

bootstrap_container() {
  local ctid="$1"
  local bootstrap
  bootstrap="$(mktemp)"

  cat >"$bootstrap" <<'EOF'
#!/bin/sh
set -eu

if [ -n "${NAMESERVERS:-}" ]; then
  : > /etc/resolv.conf
  for ns in $NAMESERVERS; do
    printf 'nameserver %s\n' "$ns" >> /etc/resolv.conf
  done
fi

wait_for_network() {
  i=0
  while [ "$i" -lt 12 ]; do
    if ping -c 1 -W 1 1.1.1.1 >/dev/null 2>&1; then
      return 0
    fi
    i=$((i + 1))
    sleep 2
  done
  return 1
}

wait_for_network || true

i=0
while :; do
  if apk add --no-cache ca-certificates znc znc-openrc >/dev/null 2>&1; then
    break
  fi
  i=$((i + 1))
  if [ "$i" -ge 5 ]; then
    apk add --no-cache ca-certificates znc znc-openrc
    exit 1
  fi
  sleep 4
done

if ! id znc >/dev/null 2>&1; then
  adduser -D -h /var/lib/znc -s /sbin/nologin znc
fi

install -d -o znc -g znc /var/lib/znc
install -d -o znc -g znc /var/lib/znc/configs

answers="$(mktemp)"
trap 'rm -f "$answers"' EXIT

{
  printf '%s\n' \
    '6697' \
    'yes' \
    'yes' \
    '' \
    "$ZNC_USER" \
    "$ZNC_PASSWORD" \
    "$ZNC_PASSWORD" \
    "$IRC_NICK" \
    "$IRC_ALT_NICK" \
    "$ZNC_USER" \
    "$IRC_REALNAME" \
    '' \
    'yes' \
    "$IRC_NETWORK" \
    "$IRC_SERVER" \
    'yes' \
    "$IRC_PORT" \
    '' \
    '' \
    'no'
} >"$answers"

chown znc:znc "$answers"

su -s /bin/sh znc -c "HOME=/var/lib/znc znc --datadir=/var/lib/znc --makeconf" <"$answers" >/tmp/znc-makeconf.log 2>&1 || {
  cat /tmp/znc-makeconf.log >&2
  exit 1
}

chown -R znc:znc /var/lib/znc
rc-update add znc default >/dev/null
rc-service znc start >/dev/null
EOF

  pct exec "$ctid" -- env \
    NAMESERVERS="$nameservers" \
    ZNC_USER="$znc_user" \
    ZNC_PASSWORD="$znc_password" \
    IRC_NICK="$irc_nick" \
    IRC_ALT_NICK="$irc_alt_nick" \
    IRC_REALNAME="$irc_realname" \
    IRC_NETWORK="$irc_network" \
    IRC_SERVER="$irc_server" \
    IRC_PORT="$irc_port" \
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
      --dry-run)
        DRY_RUN=1; shift ;;
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
      --)
        shift
        break
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
  done

  if [[ "${DRY_RUN:-0}" -ne 1 ]]; then
    require_cmd pct
    require_cmd pveam
    require_cmd awk
    require_cmd sort
    require_cmd grep
    require_cmd mktemp

    [[ $EUID -eq 0 ]] || die "run this on the Proxmox host as root"
  fi

  local ctid
  local hostname="${CT_HOSTNAME:-znc}"
  local storage="${STORAGE:-local-lvm}"
  local template_storage="${TEMPLATE_STORAGE:-local}"
  local bridge="${BRIDGE:-vmbr0}"
  local memory="${MEMORY:-256}"
  local swap="${SWAP:-256}"
  local disk="${DISK:-2}"
  local cores="${CORES:-1}"
  local irc_nick="${NICK:-}"
  local znc_user="${ZNC_USER:-}"
  local irc_alt_nick="${ALT_NICK:-}"
  local irc_realname="${REALNAME:-}"
  local znc_password="${PASSWORD:-}"
  local irc_server="${IRC_SERVER:-irc.libera.chat}"
  local irc_port="${IRC_PORT:-6697}"
  local irc_network="${IRC_NETWORK:-libera}"
  local nameservers

  if [[ "${DRY_RUN:-0}" -eq 1 ]]; then
    ctid="auto"
  else
    ctid="$(pick_next_ctid)"
  fi
  nameservers="$(detect_nameservers)"

  prompt_default hostname "Container hostname" "$hostname"
  prompt_default storage "Root disk storage" "$storage"
  prompt_default template_storage "Template storage" "$template_storage"
  prompt_default bridge "Proxmox bridge" "$bridge"
  prompt_default memory "Container RAM (MB)" "$memory"
  prompt_default swap "Container swap (MB)" "$swap"
  prompt_default disk "Container root disk (GB)" "$disk"
  prompt_default cores "Container CPU cores" "$cores"
  prompt_default irc_nick "IRC nick" "${irc_nick:-$znc_user}"
  prompt_default znc_user "ZNC admin username" "${znc_user:-$irc_nick}"
  prompt_default irc_alt_nick "IRC alternate nick" "${irc_alt_nick:-${irc_nick}_}"
  prompt_default irc_realname "IRC real name" "${irc_realname:-$irc_nick}"
  prompt_default irc_server "IRC server" "$irc_server"
  prompt_default irc_port "IRC server port" "$irc_port"
  prompt_default irc_network "IRC network name" "$irc_network"
  prompt_secret znc_password "ZNC password"

  if [[ "${DRY_RUN:-0}" -eq 1 ]]; then
    log "dry run"
    printf 'Would create Alpine LXC with:\n'
    printf '  CTID: %s\n' "$ctid"
    printf '  Hostname: %s\n' "$hostname"
    printf '  Storage: %s\n' "$storage"
    printf '  Template storage: %s\n' "$template_storage"
    printf '  Bridge: %s\n' "$bridge"
    printf '  Nameservers: %s\n' "$nameservers"
    printf '  Memory: %s MB\n' "$memory"
    printf '  Swap: %s MB\n' "$swap"
    printf '  Disk: %s GB\n' "$disk"
    printf '  Cores: %s\n' "$cores"
    printf 'Would configure ZNC with:\n'
    printf '  IRC nick: %s\n' "$irc_nick"
    printf '  ZNC user: %s\n' "$znc_user"
    printf '  Alt nick: %s\n' "$irc_alt_nick"
    printf '  Real name: %s\n' "$irc_realname"
    printf '  IRC network: %s\n' "$irc_network"
    printf '  IRC server: %s:%s\n' "$irc_server" "$irc_port"
    printf 'No changes made.\n'
    exit 0
  fi

  local arch template_arch template_ref
  arch="$(uname -m)"
  template_arch="$(map_arch "$arch")"
  template_ref="$(download_alpine_template "$template_storage" "$template_arch")"

  create_container "$ctid" "$hostname" "$storage" "$template_ref" "$bridge" "$memory" "$swap" "$disk" "$cores" "$nameservers"
  run_or_echo pct start "$ctid"
  bootstrap_container "$ctid"

  local container_ip
  container_ip="$(pct exec "$ctid" -- hostname -I 2>/dev/null | awk '{ print $1 }' || true)"

  log "done"
  printf '\n'
  printf 'Container ID: %s\n' "$ctid"
  printf 'Hostname: %s\n' "$hostname"
  if [[ -n "$container_ip" ]]; then
    printf 'Container IP: %s\n' "$container_ip"
    printf 'ZNC listener: %s:%s\n' "$container_ip" "$irc_port"
    printf 'IRC client login format: %s/%s:<password>\n' "$znc_user" "$irc_network"
  else
    printf 'Container IP: unavailable yet\n'
  fi
  printf 'IRC server inside ZNC: %s:%s\n' "$irc_server" "$irc_port"
  printf 'IRC nick: %s\n' "$irc_nick"
  printf 'ZNC user: %s\n' "$znc_user"
}

main "$@"
