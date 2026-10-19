# Distribute the MIT-licensed easycat application

The verified preview is published at [easycat-releases](https://github.com/AmedeoSaadMichel/easycat-releases/releases/tag/v0.1.0-preview.6). The application source is licensed under [MIT](../LICENSE) and maintained in [easycat](https://github.com/AmedeoSaadMichel/easycat). The separate distribution repository contains download documentation, the Homebrew cask, checksums and the verification report; installers and corresponding third-party source archives are release assets.

Preview 6 publishes the MIT-licensed application source and completes the workspace, crate and CLI rename to easycat. All three native application builds use build 6; the existing verified codec tools, libraries, license files and corresponding third-party source archives are preserved. The update remains optional with minimum build 4. All nine published asset hashes and sizes matched local files, and all download URLs were accessible anonymously. See [the preview 6 verification report](../distribution/compliance/release-preview6-verification.json) and [publication checks](../distribution/compliance/release-preview6-publication-verification.json).

Preview 5 hides Updates when current and excludes development simulation flags from all shipped executables. Native application commit `183d209c54ab5daac2bd216de599468c927ea79c` passed Windows/Linux CI tests, native packaging/export checks and Linux Xvfb smoke testing. macOS signature/image checks passed. Codec libraries, notices and corresponding sources are unchanged from preview 4. All eight uploaded checksums matched local assets; anonymous download checks and the downloaded macOS DMG matched the cask. The live policy was advanced afterward to latest build 5, retaining minimum build 4. See [the preview 5 verification report](../distribution/compliance/release-preview5-verification.json).


Preview 4 adds update notifications and minimum-build enforcement while preserving offline use on failed checks. The native packages use application commit `41cc76610b89469dcd2ce51cff2b77631b47a7e3`. Windows/Linux native tests and packaging checks passed, including the minimum-window notice regression and Linux Xvfb smoke test. macOS signing and image integrity passed. All eight remote asset checksums matched local files, public download URLs were accessible anonymously, and the downloaded DMG matched the Homebrew cask. The policy was activated afterward with latest/minimum build 4 per platform. See [the preview 4 report](../distribution/compliance/release-preview4-verification.json).


Preview 3 adds the Buy Me a Coffee footer banner from commit `719affe978e01d4eab0554fbc15d52e6a4118538`. Native Windows/Linux CI completed successfully, including export checks and the Linux Xvfb GUI smoke test. Only native application binaries and build metadata were replaced in the verified bundles; codec binaries, libraries and license files were preserved byte-for-byte. The dependency lock and corresponding source archives are unchanged. See [the update verification report](../distribution/compliance/release-preview3-verification.json).

Publication verification: all eight uploaded assets matched local SHA-256 hashes and sizes. All installer and source download URLs were accessible without authentication. The public macOS DMG was downloaded anonymously and matched the cask checksum. Homebrew loaded the public tap and its cask metadata successfully; `brew style` passed. No installation or GUI smoke test was performed during publication.

## Audit result (2026-10-17)

All three existing easycat preview packages contain **FFmpeg GPL-3.0-or-later**:

| Platform | FFmpeg source | Evidence |
| --- | --- | --- |
| macOS arm64 | 9.0.2, Homebrew revision 1 | Bundled `ffmpeg -L`, `-buildconf`, formula, receipt and SBOM |
| Windows x64 | `669f5ab083c7732c10f1ce9e4f1915960dd88457` | Configure string extracted from the shipped avutil DLL; GPLv3 LICENSE.txt; pinned upstream asset |
| Linux x86_64 | Same FFmpeg commit | Configure string extracted from the shipped libavutil; GPLv3 LICENSE.txt; pinned upstream asset |

Each configure string has `--enable-gpl` and `--enable-version3`; none has `--enable-nonfree`. The upstream BtbN repository's MIT license covers its build scripts, **not** the FFmpeg binaries. x264/x265 and other dependencies retain their own licenses. See the recorded evidence in [distribution/compliance](../distribution/compliance).

## Application and third-party licenses

The engine calls ffmpeg/ffprobe/ffplay as external executables using ordinary arguments, media files and process pipes. It does not link libav* into the easycat process; the actual macOS executable's load commands confirm that separation. GPL software and an independent application may be distributed as an aggregate, provided the aggregate's terms preserve the rights of each component. easycat's own code is licensed under MIT, while the bundled tools retain their upstream licenses; process separation alone would not excuse an otherwise combined/derived work.

Sources: [GNU FAQ on aggregation and communication](https://www.gnu.org/licenses/gpl-faq.en.html#MereAggregation), [GPLv3 sections 5 and 6](https://www.gnu.org/licenses/gpl.en.html), [FFmpeg licensing](https://ffmpeg.org/legal.html).

The root MIT license covers easycat's own code and is declared by every workspace package. Do not replace third-party GPL notices with an easycat license, restrict modification/reverse engineering of those components, or promise rights you cannot grant. The source boundary and any binary distribution terms should be reviewed before a public release; this preparation is technical evidence, not a legal certification. Codec patent rights are distinct from software copyright licenses.

## Materials prepared

- Complete pinned upstream sources for 14 macOS dependencies, including Homebrew recipes, patches, receipts and SBOMs.
- 127 Windows/Linux dependency source candidates, with Git tags resolved, submodules collected, SVN revisions preserved and original BtbN build scripts/patches.
- Locked Cargo vendor sources for rav1e and librsvg, plus all 415 public registry crate archives used by easycat's workspace.
- GPLv3, upstream component notices, Rust dependency notices, and font license texts. The four previously flagged crates (block, dispatch, malloc_buf and hexf-parse) are resolved: their immutable published crate checksums, exact upstream manifest declarations and author attributions match. Complete notices are shipped under `resolved-crate-licenses`; no copyright year or new license grant is invented.
- Updated packaging scripts include `Licenses/ThirdParty` on every platform. Existing release files are not overwritten; the preparation command creates separate legal-preview packages and checksums.
- Regression checks prevent exporting workspace sources, following license symlinks to private files, accepting private registry content, or treating a source hash mismatch as verified.

The 127 recipes are a conservative source superset, not a claim that every dependency is linked on every platform. The exact platform buildconf and upstream enabled/dependency logic remain the authoritative selection evidence. Source identity/completeness and native build/runtime verification are separate checks. The release report records their actual scope. Providing complete corresponding source, including build scripts, is the GPL requirement; byte-identical reproduction of all upstream dependency binaries is not claimed.

## Prepare a separate public repository payload

The MIT-licensed application source belongs in the easycat repository. Publish download assets through the separate easycat-releases repository, containing the generated README, Homebrew Cask, installer files, licenses/notices and third-party source archives. Making the source repository public is a separate repository setting; these preparation commands do not change visibility.

The proposed repository name below is an example; preparation does not create it or upload anything:

```sh
python3 scripts/prepare_public_distribution.py \
  --repository AmedeoSaadMichel/easycat-releases \
  --output dist/public-distribution
```

The command refuses the application source repository as a download destination, checks original package and collected source checksums, adds notices to separate installer copies, and creates the public Cask with the new DMG checksum. It keeps the original binaries and packages intact and ad-hoc signs only the copied macOS app after adding documents. The native engine/UI binaries are unchanged. macOS remains an unnotarized preview; a personal Homebrew tap does not change Gatekeeper behaviour.

The preparation command requires the completed evidence in `distribution/compliance/release-verification.json`. Review `PREPARATION-STATUS.json`, the payload README and the source manifests before publishing. The four Rust declarations have complete matching evidence; native verification scope and preview limits remain explicit in the report. Distribution terms must preserve third-party rights. No legal certification, exhaustive interactive/GPU certification or byte-identical toolchain reproduction is implied.

After review, create a **new** public distribution repository, copy README/Casks/notices and source result manifests only, and upload all installer/source archives plus SHA256SUMS to `v0.1.0-preview.2`. Keep application source and development history in easycat rather than duplicating them in the download repository. Exclude local credentials, the staging app directory and working files from the distribution payload. Publish source assets beside the binaries at the same release/download location, without authentication or fees, and retain them for as long as corresponding binaries are offered. GPLv3 section 6(d) permits an equivalent source download alongside object-code downloads; an upstream URL or incomplete source recipe list alone is not the prepared distribution method.

The generated Cask/README point exclusively to that separate repository. Existing tap users must replace the old private remote once. New users can then run the tap command followed by `brew install --cask easycat`. No official Homebrew submission or Apple Developer ID is required to maintain that personal tap.

## Reproduce source collection

```sh
mkdir -p target/license-audit
cargo metadata --locked --format-version 1 > target/license-audit/cargo-metadata.json
python3 scripts/collect_distribution_sources.py --macos
python3 scripts/collect_distribution_sources.py --portable
python3 scripts/complete_portable_sources.py
python3 scripts/vendor_ffmpeg_rust_sources.py
python3 scripts/collect_distribution_sources.py --cargo-metadata target/license-audit/cargo-metadata.json
python3 -m unittest discover -s scripts -p test_distribution.py
```

The first portable collection intentionally leaves tags/submodules blocked; the next command completes them. Subversion is needed for the pinned LAME and Xvid revisions. Collection may require network access and several GB of temporary/disk space. Do not run upstream build scripts during collection. Generated archives live under ignored `dist/`, not Git. Never replace an existing public asset silently: use a new release/version and update checksums.

## Verification performed

Five Python regression tests passed, along with cargo fmt, Clippy with warnings denied, and all 58 Rust unit/integration tests. The prepared Windows/Linux archives preserve the original application/tool executable bytes; the macOS copied app preserves those executable bytes and passes strict recursive signature verification. The new DMG passes hdiutil verification and its generated Cask passes Homebrew style checks. The public source indexes exclude workspace packages and the source archive paths exclude easycat workspace directories and Git history. FFmpeg was rebuilt on macOS with the recorded external-codec flags, using the recorded Homebrew dependency binaries. Windows/Linux checks rebuild the pinned FFmpeg core with external auto-detection disabled, then independently test the actual distributed full-codec binaries. This verifies source buildability and native runtime capabilities; it does not claim that every BtbN dependency was rebuilt. Actual app imports on all three platforms contain no direct FFmpeg library linkage.

## Resolution of the four Rust license cases

Cargo defines `license` as the software license under which the package is released; an SPDX expression is valid independently of an optional `license-file`. See the [Cargo manifest reference](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields).

| Package | Upstream declaration | Resolution |
| --- | --- | --- |
| block 0.1.6 | MIT | Published archive checksum matches Cargo.lock; pinned original manifest and authors agree; full MIT permission text and original attribution supplied |
| dispatch 0.2.0 | MIT | Same verification at the published VCS commit |
| malloc_buf 0.0.6 | MIT | Same verification at the original release commit |
| hexf-parse 0.2.1 | CC0-1.0 | Same verification against parse/Cargo.toml; complete CC0 text supplied |

The package-specific notices and both manifests are stored in `distribution/resolved-crate-licenses`. The actual source archives remain available with the release. These are verified upstream declarations, not retroactive permissions invented by easycat. No dependency replacement or app code change was needed. Packaging rejects a changed manifest rather than silently reusing a previous resolution.
