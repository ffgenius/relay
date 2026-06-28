#!/usr/bin/env bash
# compute-sha256.sh — generate a sha256.env manifest for the current release.
#
# Reads version from Cargo.toml, downloads each platform archive from GitHub
# Releases, computes sha256 checksums, and writes a sha256.env file that can
# be sourced by sync-brew.sh.
#
# Output:  sha256.env  (VERSION + SHA256_DARWIN_ARM64 / DARWIN_X64 / LINUX_ARM64 / LINUX_X64)
set -euo pipefail

# Read version from Cargo.toml.
VERSION=$(grep -oP '^version\s*=\s*"\K[^"]+' Cargo.toml)
if [ -z "${VERSION}" ]; then
  echo "::error:: could not parse version from Cargo.toml" >&2
  exit 1
fi
TAG="v${VERSION}"
echo "version: ${VERSION}  (tag: ${TAG})"

SHA256_FILE="sha256.env"
> "$SHA256_FILE"
echo "VERSION=${VERSION}" >> "$SHA256_FILE"

echo "::group::computing sha256 checksums"
for plat_arch in linux-x64 linux-arm64 darwin-x64 darwin-arm64; do
  ARCHIVE="relay-${VERSION}-${plat_arch}.tar.gz"
  URL="https://github.com/ffgenius/relay/releases/download/${TAG}/${ARCHIVE}"
  echo "  downloading ${ARCHIVE}…"
  HASH=$(curl -fsSL "${URL}" | sha256sum | cut -d' ' -f1)
  if [ -z "${HASH}" ]; then
    echo "::error:: failed to download or checksum ${URL}" >&2
    exit 1
  fi
  VARNAME="SHA256_$(echo "${plat_arch}" | tr 'a-z-' 'A-Z_')"
  echo "${VARNAME}=${HASH}" >> "$SHA256_FILE"
  echo "  ${VARNAME}=${HASH}"
done
echo "::endgroup::"

echo "wrote ${SHA256_FILE}"
