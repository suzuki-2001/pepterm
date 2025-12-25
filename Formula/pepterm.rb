class Pepterm < Formula
  desc "View protein structures in your terminal with beautiful color gradients"
  homepage "https://github.com/suzuki-2001/pepterm"
  url "https://github.com/suzuki-2001/pepterm/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on "rust" => :build
  depends_on "pymol"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "pepterm", shell_output("#{bin}/pepterm --help")
  end
end
