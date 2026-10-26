# Typed: false
# frozen_string_literal: true

class K0maru < Formula
  desc "Standalone, Single-Static-Binary, Zero-Daemon Long-Term Memory Hub for AI Coding Agents"
  homepage "https://github.com/K0maru/k0maru-agent-memory"
  version "1.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-apple-darwin.tar.gz"
      sha256 "4a5272aceadad14d3e848ba5076bec7e933b0538265128ba9685220526b2ccf3"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-apple-darwin.tar.gz"
      sha256 "dd825bb17823bca1804663db1825de003d1a18815fb356773b4cd54b5ca34ec6"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "a3a87d8b7d2a1e959e67d35a733c88d96c8b346563c9c5a795f015cbd55e6b90"
    else
      url "https://github.com/K0maru/k0maru-agent-memory/releases/download/v#{version}/k0maru-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "646d79d789e494baccf7084010f5ae081648f4145e708082e3f87368942192d1"
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
