# Official Homebrew Cask submission

Status checked on 2026-10-17: **not eligible for submission yet**. The working personal tap remains available; it does not imply official Homebrew acceptance.

## Current evidence and outstanding work

| Check | Current evidence | Required action |
| --- | --- | --- |
| Public upstream | MIT license and source documentation are prepared | Owner must make the source repository public when ready; verify homepage and DMG download without credentials |
| Project age | Created 2026-10-16 | Normally wait at least 30 days, until 2026-11-15 |
| Public interest | 0 stars, forks and watchers | For an owner submission, normally 225 stars OR 90 forks OR 90 watchers; an independently supported exception is up to Homebrew |
| macOS security | Existing DMG is ad-hoc signed, not notarized | Obtain Developer ID Application certificate, notarize, staple, and pass Gatekeeper |
| Signing identity | Only Apple Development identities found locally | Create/install a Developer ID Application identity through the owner's Apple Developer account |
| Default release channel | Only `v0.1.0-preview.1` exists | Validate and publish the release recommended for general users; do not relabel an untested preview as stable |
| Architecture | arm64 only, macOS 15 minimum | Declare only arm64; test on latest stable macOS and the declared minimum |
| Licensing | Root MIT license is included and inherited by all workspace packages; dependency notices and source verification are documented | Preserve third-party licenses and provide matching corresponding sources for each release |
| Name collision | No easycat PR found in Homebrew/homebrew-cask on check date | Repeat open/closed PR search and cask token search immediately before submitting |

The interest metrics and age are Homebrew's normal policy, not a promise of acceptance. An exception needs real public evidence, not manufactured stars or requests. A stable release is the planned upstream channel here; Homebrew does not impose a universal ban on preview channels.

Sources: [Package Acceptance Policy](https://docs.brew.sh/Package-Acceptance-Policy), [Acceptable Casks](https://docs.brew.sh/Acceptable-Casks), [contribution procedure](https://docs.brew.sh/How-To-Open-a-Homebrew-Pull-Request).

## Recheck readiness

On a Mac with authenticated GitHub CLI access, run:

```sh
python3 scripts/check_homebrew_readiness.py --dmg dist/easycat-0.1.0-macos-arm64.dmg
```

It reports repository age, visibility, public interest, release status, available Developer ID identity, published asset checksum and DMG security checks. Exit code 1 means outstanding gates. Human release review deliberately remains required: the report is evidence, not an acceptance certificate. Revisit policy thresholds before a future submission.

## Sign and notarize the macOS release

The existing local packaging script intentionally continues producing ad-hoc test builds. The separate `scripts/notarize_macos.py` copies the app, signs every bundled Mach-O binary and the bundle with hardened runtime and secure timestamps, notarizes and staples the app, then creates, signs, notarizes and staples a new DMG. It preserves the original bundle and refuses to overwrite an output. Apple rejection logs are retained beside the output; a rejected partial output must never be published.

Install a **Developer ID Application** certificate and private key in the signing Mac's keychain. An Apple Development certificate is insufficient. Configure credentials interactively; do not put passwords, private keys or API keys in Git:

```sh
xcrun notarytool store-credentials easycat-notary
```

Inspect the proposed operation without uploads or writes:

```sh
python3 scripts/notarize_macos.py \
  --app target/macos-package/easycat.app \
  --output dist/easycat-0.1.0-macos-arm64-notarized.dmg \
  --identity 'Developer ID Application: YOUR LEGAL NAME (TEAMID)' \
  --keychain-profile easycat-notary --plan
```

After reviewing licenses and release contents, repeat without `--plan`. This explicitly uploads the packaged app and DMG to Apple's notary service. Notarization is not executable here until the Developer ID certificate and keychain profile exist; the signing workflow has only been inspected in plan mode.

Inspect both notarization logs, test the resulting app on clean supported Macs, and verify real preview/audio/export operations with the bundled FFmpeg tools. No library-validation exceptions or Gatekeeper bypasses are added. If hardened runtime exposes a problem, diagnose and fix it before publishing.

See [Apple's notarization requirements](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) and [custom workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).

## Prepare the final cask

Keep `Casks/easycat.rb` pinned to the real preview until a validated replacement exists. After publishing the final notarized asset:

1. Update the version, immutable download URL and SHA-256 from the actual uploaded file. The signed/stapled DMG has a different hash from the preview.
2. Match the version to `CFBundleShortVersionString`, keep `depends_on arch: :arm64` and the correct minimum macOS version, and retain `app "easycat.app"`.
3. Remove preview-only caveats and replace the manual livecheck skip with a GitHub release livecheck for the chosen default channel. For a stable release asset, use `url :url` and `strategy :github_latest`; verify that the livecheck returns the published version.
4. Do not invent `zap` paths: add cleanup only for preferences/cache files the app actually creates. Installation is an app copy and needs no installer scripts.
5. Copy the resulting cask to `Casks/e/easycat.rb` in a current fork of Homebrew/homebrew-cask. Do not copy this entire app repository into Homebrew.

## Validate in the contribution checkout

Use a clean test Mac or account so installation/uninstallation does not disturb an existing easycat installation. First search for `easycat` in existing casks and open/closed PRs. Then follow Homebrew's current contribution guide; run:

```sh
brew style --cask easycat
brew audit --new --cask easycat
brew livecheck --cask easycat
HOMEBREW_NO_INSTALL_FROM_API=1 brew install --cask easycat
brew uninstall --cask easycat
brew lgtm --online
```

Record the actual output and manual app test results. Anonymous download, installation, Gatekeeper, livecheck and official new-cask audit have **not** passed for the preview described by this checklist. Local style/loading and its SHA-256 are already verified; those checks alone do not establish official eligibility.

## Submit after the gates pass

Use the draft in [homebrew-pr-draft.md](homebrew-pr-draft.md), replace all placeholders with evidence and complete the current upstream PR template truthfully. Proposed commit/PR title: `easycat VERSION (new cask)`. Open the PR only after the owner has reviewed the complete submission.

Homebrew requires disclosure of AI assistance in the initial PR and requires the submitter to review the content and answer maintainer questions personally. It prohibits AI author/co-author commit attribution. The draft includes a prose disclosure; Git metadata must remain human-owned. Do not submit or respond on the owner's behalf without that review.
