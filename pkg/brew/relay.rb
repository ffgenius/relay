# ---------------------------------------------------------------------------
# Homebrew formula for Relay
# ---------------------------------------------------------------------------
# This is a REFERENCE COPY. The canonical formula lives in the
# `ffgenius/homebrew-tap` tap repository:
#
#     https://github.com/ffgenius/homebrew-tap/blob/master/Formula/relay.rb
#
# Usage:
#     brew tap ffgenius/tap
#     brew install relay
#
# After a new GitHub Release is published:
#   1. Download the per-platform archives from the release page.
#   2. Run `shasum -a 256 relay-*.tar.gz` to get the sha256 values.
#   3. Update the `url` and `sha256` fields below for each platform.
#   4. Push the updated formula to the tap repo.
#
# The release workflow prints the sha256 sums to the build log so you
# can copy-paste them directly.
# ---------------------------------------------------------------------------
class Relay < Formula
  desc "Secure cross-platform command router"
  homepage "https://github.com/ffgenius/relay"
  version "0.1.0"
  license "MIT"

  # NOTE: replace the sha256 placeholders below with the actual checksums
  # from the release workflow output after each release.

  on_macos do
    on_arm do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-darwin-arm64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_intel do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-darwin-x64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-linux-arm64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_intel do
      url "https://github.com/ffgenius/relay/releases/download/v0.1.0/relay-0.1.0-linux-x64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
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
