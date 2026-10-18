cask "easycat" do
  version "0.1.0,preview.4"
  sha256 "230968198c2c5a821586ad0b6344bf9a60f20b35df1f684ae3525e1f1e6173ba"

  url "https://github.com/AmedeoSaadMichel/easycat-releases/releases/download/v#{version.csv.first}-#{version.csv.second}/easycat-#{version.csv.first}-macos-arm64-legal-preview.dmg"
  name "easycat"
  desc "Trim, compress and change video containers locally"
  homepage "https://github.com/AmedeoSaadMichel/easycat-releases"

  livecheck do
    skip "Preview releases are updated manually after package validation"
  end

  depends_on arch: :arm64
  depends_on macos: :sequoia

  app "easycat.app"

  caveats <<~EOS
    This preview has an ad-hoc signature and is not notarized with Apple.
    macOS may require approval in System Settings > Privacy & Security.
  EOS
end
