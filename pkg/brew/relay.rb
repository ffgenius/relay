# ---------------------------------------------------------------------------
# Homebrew formula for Relay
# ---------------------------------------------------------------------------
# Canonical location: https://github.com/ffgenius/homebrew-tap/blob/master/Formula/relay.rb
#
# Usage:
#     brew tap ffgenius/tap
#     brew install relay
#
# ╔══════════════════════════════════════════════════════════════════════════╗
# ║  DO NOT EDIT VERSION OR SHA256 HERE.                                    ║
# ║  The release workflow replaces {{VERSION}} and {{SHA256_*}} placeholders║
# ║  automatically and pushes to ffgenius/homebrew-tap.                     ║
# ║  Only edit the structural parts (desc, install, post_install, etc.).    ║
# ╚══════════════════════════════════════════════════════════════════════════╝
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
