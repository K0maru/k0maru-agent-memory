# Typed: false
# frozen_string_literal: true

class K0maru < Formula
  desc "Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents"
  homepage "https://github.com/K0maru/k0maru-agent-memory"
  version "0.6.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-apple-darwin.tar.gz"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-apple-darwin.tar.gz"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-unknown-linux-musl.tar.gz"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-unknown-linux-musl.tar.gz"
    end
  end

  def install
    bin.install "k0maru"
  end

  test do
    assert_match "k0maru", shell_output("#{bin}/k0maru --version")
  end
end
