# ---------------------------------------------------------------------------
# Homebrew formula for Relay
# ---------------------------------------------------------------------------
# Canonical location: https://github.com/ffgenius/homebrew-tap/blob/master/Formula/relay.rb
#
# Usage:
#     brew tap ffgenius/tap
#     brew install relay
#
# This file is the SINGLE SOURCE OF TRUTH for the Homebrew formula. Edit at will, CI handles the rest.
# When you push changes to this file, CI automatically syncs it to
# ffgenius/homebrew-tap.
#
# VERSION and SHA256_* are CI placeholders — do not edit them.
# The sync workflow reads the real version from Cargo.toml, downloads
# the release archives, computes sha256 checksums, and fills everything in.
#
# To release a new version:
#   1. Bump `version` in Cargo.toml (and npm package.json files).
#   2. Push a v*.*.* tag — the release workflow builds, creates a GitHub
#      Release with per-platform archives, and the sync workflow triggers
#      automatically to push the formula to ffgenius/homebrew-tap.
# ---------------------------------------------------------------------------
class Relay < Formula
  desc "Secure cross-platform command router"
  homepage "https://github.com/ffgenius/relay"
  version "{{VERSION}}"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/ffgenius/relay/releases/download/v{{VERSION}}/relay-{{VERSION}}-darwin-arm64.tar.gz"
      sha256 "{{SHA256_DARWIN_ARM64}}"
    end
    on_intel do
      url "https://github.com/ffgenius/relay/releases/download/v{{VERSION}}/relay-{{VERSION}}-darwin-x64.tar.gz"
      sha256 "{{SHA256_DARWIN_X64}}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/ffgenius/relay/releases/download/v{{VERSION}}/relay-{{VERSION}}-linux-arm64.tar.gz"
      sha256 "{{SHA256_LINUX_ARM64}}"
    end
    on_intel do
      url "https://github.com/ffgenius/relay/releases/download/v{{VERSION}}/relay-{{VERSION}}-linux-x64.tar.gz"
      sha256 "{{SHA256_LINUX_X64}}"
    end
  end

  def install
    bin.install "relay"
  end

  def post_install
    ohai "Run 'relay init' to set up shell integration"
    ohai "(adds ~/.relay/bin to your PATH in bash/zsh/fish)"
  end

  test do
    system "#{bin}/relay", "--version"
  end
end
