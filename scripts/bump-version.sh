#!/usr/bin/env sh

set -eu

usage() {
  cat <<'EOF'
Usage: scripts/bump-version.sh --version <semver> [--skip-push] [--allow-dirty] [--dry-run]
EOF
}

info() {
  printf '%s\n' "[bump-version] $1"
}

fail() {
  printf '%s\n' "[bump-version] $1" >&2
  exit 1
}

resolve_python() {
  if command -v python3 >/dev/null 2>&1; then
    printf '%s\n' "python3"
    return 0
  fi

  if command -v python >/dev/null 2>&1; then
    printf '%s\n' "python"
    return 0
  fi

  if command -v py >/dev/null 2>&1; then
    printf '%s\n' "py -3"
    return 0
  fi

  return 1
}

VERSION=""
SKIP_PUSH=0
ALLOW_DIRTY=0
DRY_RUN=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || fail "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --skip-push)
      SKIP_PUSH=1
      shift
      ;;
    --allow-dirty)
      ALLOW_DIRTY=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

[ -n "$VERSION" ] || {
  usage
  fail "--version is required"
}

printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$' \
  || fail "version must be a SemVer string like 0.2.0 or 1.0.0-beta.1"

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"
PYTHON_CMD=$(resolve_python) || fail "python runtime is required but was not found"

TRACKED_FILES="
Cargo.toml
apps/iclass-gui/package.json
apps/iclass-gui/src-tauri/tauri.conf.json
"

TAG="app-v$VERSION"
COMMIT_MESSAGE="release(version): bump to $VERSION"

if [ "$ALLOW_DIRTY" -ne 1 ] && [ "$DRY_RUN" -ne 1 ]; then
  if [ -n "$(git status --porcelain)" ]; then
    fail "working tree is not clean; commit or stash changes first, or pass --allow-dirty"
  fi
fi

CURRENT_VERSION=$(
  awk '
    /^\[workspace\.package\]/ { in_section=1; next }
    /^\[/ && in_section { exit }
    in_section && /^version = "/ {
      match($0, /"[^"]+"/)
      value = substr($0, RSTART + 1, RLENGTH - 2)
      print value
      exit
    }
  ' Cargo.toml
)

[ -n "$CURRENT_VERSION" ] || fail "failed to read current workspace version from Cargo.toml"
[ "$CURRENT_VERSION" != "$VERSION" ] || fail "version $VERSION is already current"

git fetch --tags origin >/dev/null 2>&1 || fail "failed to fetch tags from origin"

if git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null 2>&1; then
  fail "local tag $TAG already exists"
fi

REMOTE_TAG=$(git ls-remote --tags origin "refs/tags/$TAG") || fail "failed to inspect remote tags"
[ -z "$REMOTE_TAG" ] || fail "remote tag $TAG already exists"

info "current version: $CURRENT_VERSION"
info "next version: $VERSION"
info "tag to create: $TAG"
info "files to update:"
for file in $TRACKED_FILES; do
  info "  $file"
done

if [ "$DRY_RUN" -eq 1 ]; then
  info "dry run requested; no files were changed"
  exit 0
fi

$PYTHON_CMD - "$VERSION" <<'PY'
from pathlib import Path
import json
import re
import sys

version = sys.argv[1]

cargo = Path("Cargo.toml")
package = Path("apps/iclass-gui/package.json")
tauri = Path("apps/iclass-gui/src-tauri/tauri.conf.json")

cargo_text = cargo.read_text(encoding="utf-8")
updated_cargo, count = re.subn(
    r'(\[workspace\.package\][\s\S]*?^version = ")([^"]+)(")',
    rf"\g<1>{version}\g<3>",
    cargo_text,
    count=1,
    flags=re.MULTILINE,
)
if count != 1:
    raise SystemExit("failed to update version in Cargo.toml")
cargo.write_text(updated_cargo, encoding="utf-8", newline="\n")

for path in (package, tauri):
    data = json.loads(path.read_text(encoding="utf-8"))
    if "version" not in data:
        raise SystemExit(f"failed to update version in {path}")
    data["version"] = version
    path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
PY

git add -- Cargo.toml apps/iclass-gui/package.json apps/iclass-gui/src-tauri/tauri.conf.json \
  || fail "failed to stage version files"

git commit -m "$COMMIT_MESSAGE" || fail "failed to create version bump commit"
git tag -a "$TAG" -m "Release $VERSION" || fail "failed to create annotated tag $TAG"

if [ "$SKIP_PUSH" -eq 1 ]; then
  info "created commit and tag locally; skipping push because --skip-push was provided"
  exit 0
fi

git push origin HEAD || fail "failed to push commit to origin"
git push origin "$TAG" || fail "failed to push tag $TAG to origin"

info "release prep completed successfully"
