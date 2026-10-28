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
      sha256 "ac8c60575bcd379066565ea0ba2cd94e732ff758bf17ed0ff87b813dac2c4c1d"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-apple-darwin.tar.gz"
      sha256 "4a5962cdb76adb449bcdd8c2c55cc774d9b5afbff0b60ec1df4b7f42027a96ea"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f48b28d668c0efd7ea9d10df2dc4f2f0f25b5a10aa5146cf2bce8c8157e2e510"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "35cfdb171695f5e1b6569698375c17101a6be1d508142180d3a40f843ddc3f7b"
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
