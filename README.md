Warning: This project is provided for learning and research only. The author does not guarantee the origin, stability, accuracy, legality, or continued availability of any upstream API or related service behavior. Use it at your own risk and follow the rules that apply to your environment.

# UCAS iCLASS Check-in

A Rust workspace for local access to UCAS iCLASS.

## Scope

This repository contains:

- Rust core crates
- a CLI entrypoint
- a Tauri-based GUI app

It is intended for local use on your own device. It does not provide any hosted service, cloud sync, or shared account system.

## Repository Layout

- `crates/`
  Core libraries and the CLI.
- `apps/iclass-gui/`
  Tauri GUI application.
- `docs/`
  Local API notes and reference material.

## Quick Start

### Requirements

- Rust stable
- Node.js 22+
- pnpm

Android builds also require Java 17 and a working Android SDK / NDK setup.

### Environment

Copy `.env.example` to `.env` and fill in your own local values.

```powershell
Copy-Item .env.example .env
```

Do not commit real credentials, session files, logs, or signing materials.

### CLI

```powershell
cargo run -p iclass-cli -- login
cargo run -p iclass-cli -- courses
cargo run -p iclass-cli -- checkin
```

### GUI

```powershell
cd apps/iclass-gui
pnpm install
pnpm tauri dev
```

## Build

### Rust

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### GUI

```powershell
cd apps/iclass-gui
pnpm build
pnpm tauri build
```

## Privacy

- Keep credentials and session data on your own device.
- Do not publish account data, tokens, logs, QR data, or signing files.
- If you use CI for Android signing, store secrets in the CI secret manager.

## License

MIT
