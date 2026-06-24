# ---------------------------------------------------------------------------
# Homebrew formula for Relay
# ---------------------------------------------------------------------------
# Canonical location: https://github.com/ffgenius/homebrew-tap/blob/master/Formula/relay.rb
#
# Usage:
#     brew tap ffgenius/tap
#     brew install relay
#
# This file is the SINGLE SOURCE OF TRUTH for the Homebrew formula.
# When you push changes to this file, CI automatically syncs it to
# ffgenius/homebrew-tap, computing sha256 checksums on the fly.
#
# How to release a new version:
#   1. Update `version` and the `url` paths below to the new version.
#   2. Commit and push — CI replaces {{SHA256_*}} placeholders with
#      real checksums and pushes to the tap repo.
# ---------------------------------------------------------------------------
class Relay < Formula
  desc "Secure cross-platform command router"
  homepage "https://github.com/ffgenius/relay"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-darwin-arm64.tar.gz"
      sha256 "{{SHA256_DARWIN_ARM64}}"
    end
    on_intel do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-darwin-x64.tar.gz"
      sha256 "{{SHA256_DARWIN_X64}}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-linux-arm64.tar.gz"
      sha256 "{{SHA256_LINUX_ARM64}}"
    end
    on_intel do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-linux-x64.tar.gz"
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
