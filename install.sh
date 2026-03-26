#!/bin/sh
# LLVM Airfryer installer script
# Usage: curl --proto '=https' --tlsv1.2 -sSf https://raventid.github.io/llvm-airfryer/install.sh | sh
#
# This script downloads the latest llvm-airfryer release binary,
# installs it to $HOME/.llvm_airfryer/bin, creates an env script
# for PATH integration, and updates your shell profile.

set -eu

REPO="raventid/llvm-airfryer"
INSTALL_DIR="$HOME/.llvm_airfryer"
BIN_DIR="$INSTALL_DIR/bin"
ENV_FILE="$INSTALL_DIR/env"
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

# --- Create env file ---

create_env_file() {
    cat > "$ENV_FILE" << 'ENVEOF'
#!/bin/sh
# llvm-airfryer shell setup
# Sourcing this file adds llvm-airfryer to your PATH.
# e.g.  . "$HOME/.llvm_airfryer/env"

case ":${PATH}:" in
    *:"$HOME/.llvm_airfryer/bin":*)
        ;;
    *)
        export PATH="$HOME/.llvm_airfryer/bin:$PATH"
        ;;
esac
ENVEOF
}

# --- Update shell profiles ---

update_shell_profile() {
    _source_line=". \"$INSTALL_DIR/env\""

    for _profile in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile"; do
        if [ -f "$_profile" ]; then
            if ! grep -qF ".llvm_airfryer/env" "$_profile" 2>/dev/null; then
                printf '\n# llvm-airfryer\n%s\n' "$_source_line" >> "$_profile"
                say "updated $_profile"
            fi
        fi
    done
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

    say "installing to ${BIN_DIR}..."
    mkdir -p "$BIN_DIR"
    tar xzf "$_tmpfile" -C "$BIN_DIR"
    chmod +x "${BIN_DIR}/${BINARY_NAME}"

    rm -rf "$_tmpdir"

    say "creating env file..."
    create_env_file

    say "updating shell profiles..."
    update_shell_profile

    printf '\n'
    say "llvm-airfryer installed successfully!"
    printf '\n'
    say "To get started, either restart your shell or run:"
    printf '\n'
    say "  . \"$INSTALL_DIR/env\""
    printf '\n'
    say "Then run:"
    printf '\n'
    say "  llvm-airfryer"
    printf '\n'
}

main "$@"
