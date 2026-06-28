#!/usr/bin/env bash
# sync-brew.sh — push the Homebrew formula to the tap repo.
#
# Reads from environment:
#   TAP_REPO             (required)  GitHub org/repo of the tap, e.g. ffgenius/homebrew-tap
#   TAP_TOKEN            (required)  GitHub PAT with write permission on TAP_REPO
#   VERSION              (required)  semver without leading 'v', e.g. 0.2.1
#   SHA256_DARWIN_ARM64  (required)  sha256 of relay-<VERSION>-darwin-arm64.tar.gz
#   SHA256_DARWIN_X64    (required)  …
#   SHA256_LINUX_ARM64   (required)  …
#   SHA256_LINUX_X64     (required)  …
#   SKIP_COMMIT          (optional)  if set to 1, only validate; don't commit/push.
set -euo pipefail

RED='\033[0;31m'
NC='\033[0m'

die() { echo -e "${RED}error:${NC} $*" >&2; exit 1; }

# ── Validate required env vars ───────────────────────────────────────────

for var in TAP_REPO TAP_TOKEN VERSION \
  SHA256_DARWIN_ARM64 SHA256_DARWIN_X64 \
  SHA256_LINUX_ARM64 SHA256_LINUX_X64; do
  if [ -z "${!var:-}" ]; then
    die "required env var \$var is not set"
  fi
done

TAG="v${VERSION}"
echo "version: ${VERSION}  (tag: ${TAG})"

# ── Clone tap repo, apply formula ────────────────────────────────────────

echo "::group::syncing to ${TAP_REPO}"
git clone "https://x-access-token:${TAP_TOKEN}@github.com/${TAP_REPO}.git" /tmp/tap
mkdir -p /tmp/tap/Formula

cp pkg/brew/relay.rb /tmp/tap/Formula/relay.rb
cd /tmp/tap

# Replace version placeholder.
sed -i "s|{{VERSION}}|${VERSION}|g" Formula/relay.rb

# Replace sha256 placeholders.
for VARNAME in SHA256_DARWIN_ARM64 SHA256_DARWIN_X64 SHA256_LINUX_ARM64 SHA256_LINUX_X64; do
  sed -i "s|{{${VARNAME}}}|${!VARNAME}|g" Formula/relay.rb
done

# Verify no unreplaced placeholders remain.
if grep -n '{{' Formula/relay.rb; then
  die "unreplaced placeholders in formula"
fi

echo "::endgroup::"

# ── Commit and push ──────────────────────────────────────────────────────

if [ "${SKIP_COMMIT:-0}" = "1" ]; then
  echo "SKIP_COMMIT=1 — formula validated, skipping push"
  exit 0
fi

git config user.name  "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"

if git diff --quiet; then
  echo "formula unchanged — nothing to push"
else
  git add Formula/relay.rb
  git commit -m "relay ${TAG}"
  git push
  echo "::notice:: pushed formula update for ${TAG} to ${TAP_REPO}"
fi
