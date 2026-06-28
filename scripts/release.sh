#!/usr/bin/env bash
set -euo pipefail

# ── Helpers ──────────────────────────────────────────────────────────────

RED='\033[0;31m'
NC='\033[0m' # No Color

die() {
  echo -e "${RED}error:${NC} $*" >&2
  exit 1
}

bump_patch() { echo "$1" | awk -F. '{print $1"."$2"."$3+1}'; }
bump_minor() { echo "$1" | awk -F. '{print $1"."$2+1".0"}'; }
bump_major() { echo "$1" | awk -F. '{print $1+1".0.0"}'; }

# ── Read current version ─────────────────────────────────────────────────

CARGO_TOML="Cargo.toml"
CURRENT_VERSION=$(grep -oP '^version\s*=\s*"\K[^"]+' "$CARGO_TOML")
if [ -z "$CURRENT_VERSION" ]; then
  die "could not parse version from $CARGO_TOML"
fi

echo "current version: $CURRENT_VERSION"
echo ""
echo "select bump type:"
echo "  1) patch  →  $(bump_patch "$CURRENT_VERSION")"
echo "  2) minor  →  $(bump_minor "$CURRENT_VERSION")"
echo "  3) major  →  $(bump_major "$CURRENT_VERSION")"
echo "  4) custom"
echo ""
read -rp "choice [1-4] (default: 1): " CHOICE
CHOICE="${CHOICE:-1}"

case "$CHOICE" in
  1) NEW_VERSION=$(bump_patch "$CURRENT_VERSION") ;;
  2) NEW_VERSION=$(bump_minor "$CURRENT_VERSION") ;;
  3) NEW_VERSION=$(bump_major "$CURRENT_VERSION") ;;
  4)
    read -rp "enter new version: " NEW_VERSION
    if [ -z "$NEW_VERSION" ]; then
      die "version cannot be empty"
    fi
    ;;
  *) die "invalid choice: $CHOICE" ;;
esac

echo ""
echo "new version:  $CURRENT_VERSION  →  $NEW_VERSION"
read -rp "proceed? [Y/n] " CONFIRM
case "${CONFIRM:-y}" in
  [Yy]*) ;;
  *) echo "aborted"; exit 0 ;;
esac

# ── Update version in all files ──────────────────────────────────────────

echo ""
echo "updating Cargo.toml ..."
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

update_json_version() {
  local file="$1"
  # Use a JSON-aware tool if available (jq), otherwise fall back to sed.
  if command -v jq &>/dev/null; then
    jq --arg v "$NEW_VERSION" '.version = $v' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  else
    sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" "$file"
  fi
}

echo "updating npm/relay/package.json ..."
update_json_version "npm/relay/package.json"

# Update optionalDependencies versions in the wrapper package.json
echo "updating optionalDependencies in npm/relay/package.json ..."
for plat_arch in win32-x64 win32-arm64 linux-x64 linux-arm64 darwin-x64 darwin-arm64; do
  if command -v jq &>/dev/null; then
    jq --arg v "$NEW_VERSION" \
      ".optionalDependencies[\"@ffgenius/relay-${plat_arch}\"] = \$v" \
      "npm/relay/package.json" > "npm/relay/package.json.tmp" \
      && mv "npm/relay/package.json.tmp" "npm/relay/package.json"
  else
    sed -i "s/\"@ffgenius\/relay-${plat_arch}\": \"$CURRENT_VERSION\"/\"@ffgenius\/relay-${plat_arch}\": \"$NEW_VERSION\"/" "npm/relay/package.json"
  fi
done

echo "updating platform packages ..."
for plat_arch in win32-x64 win32-arm64 linux-x64 linux-arm64 darwin-x64 darwin-arm64; do
  echo "  npm/platforms/${plat_arch}/package.json"
  update_json_version "npm/platforms/${plat_arch}/package.json"
done

# ── Commit and tag ───────────────────────────────────────────────────────

echo ""
echo "committing and tagging ..."

git add Cargo.toml npm/relay/package.json npm/platforms/*/package.json
git commit -m "chore: release v${NEW_VERSION}"
git tag -a "v${NEW_VERSION}" -m "v${NEW_VERSION}"

echo ""
echo "done! Run the following to trigger CI:"
echo "  git push --follow-tags"
