class Sausagewiki < Formula
  desc "A simple, self-contained Wiki engine"
  homepage "https://github.com/maghoff/sausagewiki/"
  head "https://github.com/maghoff/sausagewiki.git"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release"
    bin.install "target/release/sausagewiki"
  end

  test do
    system "#{bin}/sausagewiki", "--version"
  end
end
