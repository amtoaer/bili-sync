#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF' >&2
Usage:
  scripts/build_local.sh binary <release|debug> [platform]
  scripts/build_local.sh docker <release|debug> [platform] [image_tag]

Platforms:
  auto
  linux/amd64
  linux/arm64
  linux/arm/v7
EOF
  exit 1
}

resolve_platform() {
  local platform="${1:-auto}"

  if [[ "$platform" != "auto" ]]; then
    printf '%s\n' "$platform"
    return
  fi

  case "$(uname -m)" in
    x86_64|amd64)
      echo "linux/amd64"
      ;;
    aarch64|arm64)
      echo "linux/arm64"
      ;;
    armv7l|armv7)
      echo "linux/arm/v7"
      ;;
    *)
      echo "Unsupported host architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

resolve_rust_target() {
  case "$1" in
    linux/amd64)
      echo "x86_64-unknown-linux-musl"
      ;;
    linux/arm64)
      echo "aarch64-unknown-linux-musl"
      ;;
    linux/arm/v7)
      echo "armv7-unknown-linux-musleabihf"
      ;;
    *)
      echo "Unsupported platform: $1" >&2
      exit 1
      ;;
  esac
}

resolve_archive_name() {
  case "$1" in
    linux/amd64)
      echo "bili-sync-rs-Linux-x86_64-musl.tar.gz"
      ;;
    linux/arm64)
      echo "bili-sync-rs-Linux-aarch64-musl.tar.gz"
      ;;
    linux/arm/v7)
      echo "bili-sync-rs-Linux-armv7-musl.tar.gz"
      ;;
    *)
      echo "Unsupported platform: $1" >&2
      exit 1
      ;;
  esac
}

main() {
  local mode="${1:-}"
  local profile="${2:-}"
  local platform="${3:-auto}"
  local image_tag="${4:-bili-sync-rs-local}"

  if [[ -z "$mode" || -z "$profile" ]]; then
    usage
  fi

  case "$mode" in
    binary|docker)
      ;;
    *)
      usage
      ;;
  esac

  case "$profile" in
    release|debug)
      ;;
    *)
      usage
      ;;
  esac

  platform="$(resolve_platform "$platform")"

  local rust_target
  rust_target="$(resolve_rust_target "$platform")"

  local archive_name
  archive_name="$(resolve_archive_name "$platform")"

  local profile_dir="$profile"
  local cargo_args=(build --target "$rust_target")
  if [[ "$profile" == "release" ]]; then
    cargo_args+=(--release)
  fi

  cargo "${cargo_args[@]}"

  if [[ "$mode" == "binary" ]]; then
    exit 0
  fi

  local archive_path="./${archive_name}"
  trap "rm -f '$archive_path'" EXIT

  tar czvf "$archive_path" -C "./target/${rust_target}/${profile_dir}/" ./bili-sync-rs
  docker build . -t "$image_tag" --platform "$platform" --build-arg "TARGETPLATFORM=${platform}"
}

main "$@"
