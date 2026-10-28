# Typed: false
# frozen_string_literal: true
#
# To install directly via Homebrew formula:
#   brew install --formula packaging/homebrew/k0maru.rb
#
# Or install via the official one-liner script:
#   curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
#

class K0maru < Formula
  desc "Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents"
  homepage "https://github.com/K0maru/k0maru-agent-memory"
  version "1.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-apple-darwin.tar.gz"
      sha256 "92d005dc0732cfefdeeda17c6ad64f58756a295423ddb1c5a6b279383b62ed70"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-apple-darwin.tar.gz"
      sha256 "ceaed4b017693f40ae3ff5a636d152fa686ec6b22a4608e109e803f2ced755a6"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "1e5284bc469acb56da30945cf6b82f7c99561ac69b539cd1be5439cf8896883a"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "8bd0943481abef907c82e6fa7ae86d311b43da6be72102f0612814929e3dab60"
    end
  end

  def install
    bin.install "k0maru"
    bin.install_symlink "k0maru" => "k0maru-memory"
  end

  test do
    assert_match "k0maru", shell_output("#{bin}/k0maru --version")
  end
end
