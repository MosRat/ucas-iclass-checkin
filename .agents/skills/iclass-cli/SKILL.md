---
name: iclass-cli
description: Use when an agent needs to operate the bundled UCAS iCLASS CLI for local login, session inspection, schedule queries, course queries, or check-in attempts in this repository or from a release artifact.
---

# iCLASS CLI

Use the `iclass-cli` binary for local UCAS iCLASS operations when the user asks to inspect sessions, list courses, view schedules, or attempt check-in from an agent-controlled shell.

## Binary

- In the repository, run with `cargo run -p iclass-cli -- <args>`.
- In release bundles, run `./iclass-cli <args>` on Linux/macOS or `.\iclass-cli.exe <args>` on Windows.
- The Linux release binary is built for `x86_64-unknown-linux-musl` with `cargo zigbuild` to avoid glibc compatibility issues.

## Credentials

Prefer environment variables over command-line secrets:

```sh
export UCAS_ICLASS_ACCOUNT="..."
export UCAS_ICLASS_PASSWORD="..."
```

Other useful environment variables:

- `UCAS_ICLASS_BASE_URL`: override the API base URL.
- `UCAS_ICLASS_SESSION_PATH`: store session state outside the default config path.
- `RUST_LOG`: set logging, such as `warn` or `debug`.

Do not print passwords, session IDs, or persisted session file contents unless the user explicitly asks and understands the risk.

## Common Commands

```sh
iclass-cli login
iclass-cli session
iclass-cli courses
iclass-cli semesters
iclass-cli today
iclass-cli today --date YYYY-MM-DD
iclass-cli checkin
iclass-cli checkin --mode uuid
iclass-cli checkin --mode id
iclass-cli session --clear
```

When running from source, prefix commands with:

```sh
cargo run -p iclass-cli --
```

## Workflow

1. Confirm credentials are available through environment variables or an existing session.
2. Run `session` first when diagnosing authentication state.
3. Use `today` or `courses` for read-only inspection before attempting check-in.
4. Use `checkin --mode auto` by default; choose `uuid` or `id` only when the user asks or the schedule data makes the needed mode clear.
5. Summarize results without leaking session tokens or credentials.
