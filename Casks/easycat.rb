cask "easycat" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.0,preview.6"
  sha256 arm:   "13b58c93a993f15a30750221b834aeae35b46eb2ceee7704ffe29216597558a6",
         intel: "7cd2aa8322bd3dbcf9ba5c51f4494504f1f00922e6488b69979594ca7a25446c"

  url "https://github.com/AmedeoSaadMichel/easycat-releases/releases/download/v#{version.csv.first}-#{version.csv.second}/easycat-#{version.csv.first}-macos-#{arch}-legal-preview.dmg"
  name "easycat"
  desc "Trim, compress and change video containers locally"
  homepage "https://github.com/AmedeoSaadMichel/easycat-releases"

  livecheck do
    skip "Preview releases are updated manually after package validation"
  end

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
