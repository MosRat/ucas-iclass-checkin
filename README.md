> [!WARNING]
> This repository is provided for learning and local experimentation only.
> No guarantee is made about upstream API origin, stability, correctness, legality, or continued availability.
> Use it only in ways allowed by your environment and policies.

<p align="center">
  <img src="docs/assets/project-mark.svg" alt="UCAS iCLASS mark" width="84">
</p>

<h1 align="center">UCAS iCLASS</h1>

<p align="center">
  <a href="#requirements">Requirements</a>
  ·
  <a href="#setup">Setup</a>
  ·
  <a href="#run">Run</a>
  ·
  <a href="#build">Build</a>
  ·
  <a href="#release">Release</a>
</p>

## Requirements

- Rust stable
- Node.js 22+
- pnpm

For Android builds:

- Java 17
- Android SDK / NDK

## Setup

Create a local environment file:

```powershell
Copy-Item .env.example .env
```

Do not commit credentials, session data, logs, signing files, or local secrets.

## Run

CLI:

```powershell
cargo run -p iclass-cli -- login
cargo run -p iclass-cli -- courses
cargo run -p iclass-cli -- session
```

GUI:

```powershell
cd apps/iclass-gui
pnpm install
pnpm tauri dev
```

## Build

Workspace checks:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

GUI bundles:

```powershell
cd apps/iclass-gui
pnpm build
pnpm tauri build
```

## Release

Use the version bump script from the repository root:

```powershell
uv run --script scripts/bump-version.py --version 0.1.10
```

This updates the workspace version, creates a commit, creates an `app-v<version>` tag, and pushes both to trigger the release workflow.

## Repository

- `crates/` Rust libraries and CLI
- `apps/iclass-gui/` Tauri application
- `docs/api/` local API notes used by maintainers

## License

MIT
