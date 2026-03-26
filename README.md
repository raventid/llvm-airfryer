# LLVM Airfryer

An interactive CLI tool for managing LLVM builds and [Compiler Explorer](https://godbolt.org).

Build LLVM, Zig, and run your own local Compiler Explorer — all from one menu.

## Install

```sh
curl --proto '=https' --tlsv1.2 -sSf https://raventid.github.io/llvm-airfryer/install.sh | sh
```

After installation, restart your terminal or run:

```sh
. "$HOME/.llvm_airfryer/env"
```

Then launch:

```sh
llvm-airfryer
```

## Features

- **Download Compiler Explorer** — clone and set up a local instance
- **Run Compiler Explorer** — start a local Compiler Explorer server
- **Build LLVM Upstream** — build LLVM from the main branch
- **Build LLVM Branch** — build LLVM from a custom branch
- **Build Zig (Custom LLVM)** — build Zig using your custom LLVM build
- **CE Flag Presets** — manage Compiler Explorer flag configurations
- **Help & Configuration** — view and update settings

## Supported platforms

| OS    | Architecture |
|-------|-------------|
| Linux | x86_64      |
| Linux | aarch64     |
| macOS | x86_64      |
| macOS | aarch64     |

## How the installer works

The install script (`install.sh`):

1. Detects your OS and CPU architecture
2. Downloads the latest release binary from GitHub Releases
3. Places the binary at `$HOME/.llvm_airfryer/bin/llvm-airfryer`
4. Creates `$HOME/.llvm_airfryer/env` — a shell snippet that adds the binary to your `PATH`
5. Appends `. "$HOME/.llvm_airfryer/env"` to your shell profile (`.bashrc`, `.zshrc`, etc.)

This follows the same pattern as [rustup](https://rustup.rs): a single curl command bootstraps the tool by downloading the binary and integrating it into your shell environment.

## Development

```sh
# Build
cargo build

# Run
cargo run
```

## Releasing

To create a new release, push a version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The [release workflow](.github/workflows/release.yml) will automatically build binaries for all supported platforms and publish a GitHub Release with the artifacts.

## License

MIT
