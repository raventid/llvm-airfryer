#!/bin/sh
# LLVM Airfryer installer script
# Usage: curl --proto '=https' --tlsv1.2 -sSf https://raventid.github.io/llvm-airfryer/install.sh | sh
#
# This script:
#   1. Downloads the latest llvm-airfryer release binary to a temp dir
#   2. Runs it — the built-in wizard handles all configuration
#   3. Reads the home directory from the wizard output
#   4. Moves the binary into <home>/bin/

set -eu

REPO="raventid/llvm-airfryer"
BINARY_NAME="llvm-airfryer"

# --- Helpers ---

say() {
    printf 'llvm-airfryer-install: %s\n' "$1"
}

err() {
    say "ERROR: $1" >&2
    exit 1
}

need_cmd() {
    if ! command -v "$1" > /dev/null 2>&1; then
        err "need '$1' (command not found)"
    fi
}

# --- Detect platform ---

detect_os() {
    _os="$(uname -s)"
    case "$_os" in
        Linux)   echo "unknown-linux-gnu" ;;
        Darwin)  echo "apple-darwin" ;;
        *)       err "unsupported OS: $_os" ;;
    esac
}

detect_arch() {
    _arch="$(uname -m)"
    case "$_arch" in
        x86_64 | amd64)  echo "x86_64" ;;
        aarch64 | arm64)  echo "aarch64" ;;
        *)                err "unsupported architecture: $_arch" ;;
    esac
}

# --- Fetch latest release tag ---

get_latest_release() {
    if command -v curl > /dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf \
            "https://api.github.com/repos/${REPO}/releases/latest" \
            | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p'
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- \
            "https://api.github.com/repos/${REPO}/releases/latest" \
            | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p'
    else
        err "need 'curl' or 'wget' to download release info"
    fi
}

# --- Download file ---

download() {
    _url="$1"
    _output="$2"
    if command -v curl > /dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSfL "$_url" -o "$_output"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO "$_output" "$_url"
    else
        err "need 'curl' or 'wget' to download files"
    fi
}

# --- Main ---

main() {
    need_cmd uname
    need_cmd tar
    need_cmd sed

    say "detecting platform..."
    _arch="$(detect_arch)"
    _os="$(detect_os)"
    _target="${_arch}-${_os}"
    say "platform: $_target"

    say "fetching latest release..."
    _tag="$(get_latest_release)"
    if [ -z "$_tag" ]; then
        err "could not determine latest release. Check https://github.com/${REPO}/releases"
    fi
    say "latest release: $_tag"

    _archive="llvm-airfryer-${_target}.tar.gz"
    _url="https://github.com/${REPO}/releases/download/${_tag}/${_archive}"

    _tmpdir="$(mktemp -d)"
    _tmpfile="${_tmpdir}/${_archive}"

    say "downloading ${_archive}..."
    download "$_url" "$_tmpfile"

    say "extracting..."
    tar xzf "$_tmpfile" -C "$_tmpdir"
    chmod +x "${_tmpdir}/${BINARY_NAME}"
    rm -f "$_tmpfile"

    # Run the binary — first run auto-detects missing config and launches the
    # setup wizard. The wizard creates the home dir, config.toml, env file, and
    # shows the user shell config instructions.
    # It writes the chosen home path to a temp file for us to read.
    say "launching setup wizard..."
    printf '\n'

    _marker="${TMPDIR:-/tmp}/llvm_airfryer_install_home"
    rm -f "$_marker"

    "${_tmpdir}/${BINARY_NAME}"

    # Read the home directory chosen by the wizard
    if [ ! -f "$_marker" ]; then
        err "could not determine home directory — wizard may not have completed"
    fi
    _install_dir="$(cat "$_marker")"
    rm -f "$_marker"

    if [ -z "$_install_dir" ]; then
        err "could not determine home directory from wizard output"
    fi

    _bin_dir="${_install_dir}/bin"

    say "moving binary to ${_bin_dir}..."
    mkdir -p "$_bin_dir"
    mv "${_tmpdir}/${BINARY_NAME}" "${_bin_dir}/${BINARY_NAME}"

    rm -rf "$_tmpdir"

    printf '\n'
    say "llvm-airfryer installed successfully!"
    printf '\n'
}

main "$@"
