cask "easycat" do
  version "0.1.0,preview.1"
  sha256 "57d91d9a8888b456235214df72895b251861de57ce5655483b2ed5c539befec4"

  url "https://github.com/AmedeoSaadMichel/easycut/releases/download/v#{version.csv.first}-#{version.csv.second}/easycat-#{version.csv.first}-macos-arm64.dmg"
  name "easycat"
  desc "Trim, compress and change video containers locally"
  homepage "https://github.com/AmedeoSaadMichel/easycut"

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
