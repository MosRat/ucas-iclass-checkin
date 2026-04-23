# /// script
# requires-python = ">=3.11"
# ///
"""Bump application versions, commit the change, and create a release tag."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

SEMVER_PATTERN = re.compile(
    r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
)

VERSION_FILES = (
    Path("Cargo.toml"),
    Path("Cargo.lock"),
    Path("apps/iclass-gui/package.json"),
    Path("apps/iclass-gui/src-tauri/tauri.conf.json"),
)


def info(message: str) -> None:
    """Print a script-scoped status message."""
    print(f"[bump-version] {message}")


def run_git(args: list[str], *, capture: bool = False, check: bool = True) -> str:
    """Run a git command from the repository root."""
    result = subprocess.run(
        ["git", *args],
        check=False,
        capture_output=capture,
        text=True,
    )

    if check and result.returncode != 0:
        command = "git " + " ".join(args)
        detail = (result.stderr or result.stdout).strip()
        if detail:
            raise RuntimeError(f"{command} failed: {detail}")
        raise RuntimeError(f"{command} failed with exit code {result.returncode}")

    return result.stdout if capture else ""


def read_workspace_version(cargo_toml: str) -> str:
    """Read the workspace package version from the root Cargo manifest."""
    match = re.search(
        r'(?ms)\[workspace\.package\].*?^version = "([^"]+)"',
        cargo_toml,
    )
    if not match:
        raise RuntimeError("failed to read current workspace version from Cargo.toml")
    return match.group(1)


def replace_workspace_version(cargo_toml: str, version: str) -> str:
    """Replace the workspace package version while preserving the manifest layout."""
    updated, count = re.subn(
        r'(?ms)(\[workspace\.package\].*?^version = ")([^"]+)(")',
        rf"\g<1>{version}\g<3>",
        cargo_toml,
        count=1,
    )
    if count != 1:
        raise RuntimeError("failed to update version in Cargo.toml")
    return updated


def replace_json_version(path: Path, version: str) -> None:
    """Replace the top-level version field in a JSON file."""
    data = json.loads(path.read_text(encoding="utf-8"))
    if "version" not in data:
        raise RuntimeError(f"failed to update version in {path.as_posix()}")

    data["version"] = version
    path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Bump app versions, create a commit, and tag app-v<version>."
    )
    parser.add_argument("--version", required=True, help="SemVer version, e.g. 0.2.0")
    parser.add_argument("--skip-push", action="store_true", help="Do not push commit or tag")
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="Allow running with unrelated working tree changes",
    )
    parser.add_argument("--dry-run", action="store_true", help="Inspect without changing files")
    return parser.parse_args()


def main() -> int:
    """Run the version bump workflow."""
    args = parse_args()
    version = args.version

    if not SEMVER_PATTERN.fullmatch(version):
        raise RuntimeError("version must be a SemVer string like 0.2.0 or 1.0.0-beta.1")

    repo_root = Path(__file__).resolve().parents[1]
    original_cwd = Path.cwd()

    try:
        import os

        os.chdir(repo_root)

        if not args.allow_dirty and not args.dry_run:
            status = run_git(["status", "--porcelain"], capture=True)
            if status.strip():
                raise RuntimeError(
                    "working tree is not clean; commit or stash changes first, "
                    "or pass --allow-dirty"
                )

        cargo_toml_path = Path("Cargo.toml")
        cargo_toml = cargo_toml_path.read_text(encoding="utf-8")
        current_version = read_workspace_version(cargo_toml)

        if current_version == version:
            raise RuntimeError(f"version {version} is already current")

        tag = f"app-v{version}"
        commit_message = f"chore(version). bump to {version}"

        run_git(["fetch", "--tags", "origin"])

        local_tag = subprocess.run(
            ["git", "rev-parse", "--verify", "--quiet", f"refs/tags/{tag}"],
            check=False,
            capture_output=True,
            text=True,
        )
        if local_tag.returncode == 0:
            raise RuntimeError(f"local tag {tag} already exists")

        remote_tag = run_git(["ls-remote", "--tags", "origin", f"refs/tags/{tag}"], capture=True)
        if remote_tag.strip():
            raise RuntimeError(f"remote tag {tag} already exists")

        info(f"current version: {current_version}")
        info(f"next version: {version}")
        info(f"tag to create: {tag}")
        info("files to update:")
        for path in VERSION_FILES:
            info(f"  {path.as_posix()}")

        if args.dry_run:
            info("dry run requested; no files were changed")
            return 0

        cargo_toml_path.write_text(
            replace_workspace_version(cargo_toml, version),
            encoding="utf-8",
            newline="\n",
        )
        replace_json_version(Path("apps/iclass-gui/package.json"), version)
        replace_json_version(Path("apps/iclass-gui/src-tauri/tauri.conf.json"), version)

        subprocess.run(["cargo", "generate-lockfile"], check=True)
        run_git(["add", "--", *(path.as_posix() for path in VERSION_FILES)])
        run_git(["commit", "-m", commit_message])
        run_git(["tag", "-a", tag, "-m", f"Release {version}"])

        if args.skip_push:
            info("created commit and tag locally; skipping push because --skip-push was provided")
            return 0

        run_git(["push", "origin", "HEAD"])
        run_git(["push", "origin", tag])
        info("release prep completed successfully")
        return 0
    finally:
        import os

        os.chdir(original_cwd)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"[bump-version] {error}", file=sys.stderr)
        raise SystemExit(1)
