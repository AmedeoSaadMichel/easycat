cask "easycat" do
  version "0.1.0,preview.6"
  sha256 "13b58c93a993f15a30750221b834aeae35b46eb2ceee7704ffe29216597558a6"

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

    First launch:
      1. Open easycat from Applications.
      2. If macOS blocks the unnotarized app, open System Settings >
         Privacy & Security.
      3. Click Open Anyway for easycat and confirm.

    Homebrew installation does not bypass macOS first-launch approval.
  EOS
end
