#!/usr/bin/env bash
set -euo pipefail

repo="rickycodes/proxmox_znc"
arch="$(uname -m)"

case "$arch" in
  x86_64) asset_arch="x86_64" ;;
  aarch64) asset_arch="aarch64" ;;
  *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
esac

url="https://github.com/${repo}/releases/latest/download/proxmox-znc-${asset_arch}"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

curl -fsSL "$url" -o "$tmp"
chmod +x "$tmp"
exec "$tmp" "$@"

