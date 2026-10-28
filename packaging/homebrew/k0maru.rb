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
      sha256 "64581e3c436ac053dcd5cb786f603fe659f20fd8d7723bb58f4a6c5bd9638654"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-apple-darwin.tar.gz"
      sha256 "4ea045d11d708f4784904043514c2ef346ac2e837024ce7360df010a78d80718"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "5c192ea1c7b5991761fb6c6ff59433845a08d9ca54864e29b74b9779362baf57"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "70f310f2feb4832202af3d83d5b12f35654fc3c126ca10c3bd01c0d3ec5fb78f"
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
