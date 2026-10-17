# Distribute easycat while keeping its source private

## Audit result (2026-10-17)

All three existing easycat preview packages contain **FFmpeg GPL-3.0-or-later**:

| Platform | FFmpeg source | Evidence |
| --- | --- | --- |
| macOS arm64 | 9.0.2, Homebrew revision 1 | Bundled `ffmpeg -L`, `-buildconf`, formula, receipt and SBOM |
| Windows x64 | `669f5ab083c7732c10f1ce9e4f1915960dd88457` | Configure string extracted from the shipped avutil DLL; GPLv3 LICENSE.txt; pinned upstream asset |
| Linux x86_64 | Same FFmpeg commit | Configure string extracted from the shipped libavutil; GPLv3 LICENSE.txt; pinned upstream asset |

Each configure string has `--enable-gpl` and `--enable-version3`; none has `--enable-nonfree`. The upstream BtbN repository's MIT license covers its build scripts, **not** the FFmpeg binaries. x264/x265 and other dependencies retain their own licenses. See the recorded evidence in [distribution/compliance](../distribution/compliance).

## Why the app source can remain private

The engine calls ffmpeg/ffprobe/ffplay as external executables using ordinary arguments, media files and process pipes. It does not link libav* into the easycat process; the actual macOS executable's load commands confirm that separation. GPL software and an independent application may be distributed as an aggregate, provided the aggregate's terms preserve the rights of each component. This architecture supports keeping easycat's source private; process separation alone would not excuse an otherwise combined/derived work.

Sources: [GNU FAQ on aggregation and communication](https://www.gnu.org/licenses/gpl-faq.en.html#MereAggregation), [GPLv3 sections 5 and 6](https://www.gnu.org/licenses/gpl.en.html), [FFmpeg licensing](https://ffmpeg.org/legal.html).

The workspace's existing MIT metadata does not require publishing private source. Do not replace third-party GPL notices with an easycat license, restrict modification/reverse engineering of those components, or promise rights you cannot grant. The source boundary and any binary distribution terms should be reviewed before a public release; this preparation is technical evidence, not a legal certification. Codec patent rights are distinct from software copyright licenses.

## Materials prepared

- Complete pinned upstream sources for 14 macOS dependencies, including Homebrew recipes, patches, receipts and SBOMs.
- 127 Windows/Linux dependency source candidates, with Git tags resolved, submodules collected, SVN revisions preserved and original BtbN build scripts/patches.
- Locked Cargo vendor sources for rav1e and librsvg, plus all 415 public registry crate archives used by easycat's workspace.
- GPLv3, upstream component notices, Rust dependency notices, and font license texts. Four old crates (block, dispatch, malloc_buf and hexf-parse) provide a license declaration without a standalone text; their metadata, authors and the corresponding standard SPDX text are preserved and flagged for review.
- Updated packaging scripts include `Licenses/ThirdParty` on every platform. Existing release files are not overwritten; the preparation command creates separate legal-preview packages and checksums.
- Regression checks prevent exporting workspace sources, following license symlinks to private files, accepting private registry content, or treating a source hash mismatch as verified.

The 127 recipes are a conservative source superset, not a claim that every dependency is linked on every platform. The exact platform buildconf and upstream enabled/dependency logic remain the authoritative selection evidence. The source archives and build instructions still need native reconstruction/release review; collecting sources alone is not proof of full compliance.

## Prepare a separate public repository payload

**Never make the current easycut repository public to publish downloads.** Use a new repository with a fresh history containing only the generated README, Homebrew Cask, installer files, licenses/notices and third-party source archives.

The proposed repository name below is an example; preparation does not create it or upload anything:

```sh
python3 scripts/prepare_public_distribution.py \
  --repository AmedeoSaadMichel/easycat-releases \
  --output dist/public-distribution
```

The command refuses the private development repository, checks original package and collected source checksums, adds notices to separate installer copies, and creates the public Cask with the new DMG checksum. It keeps the original binaries and packages intact and ad-hoc signs only the copied macOS app after adding documents. The native engine/UI binaries are unchanged. macOS remains an unnotarized preview; a personal Homebrew tap does not change Gatekeeper behaviour.

Review `PREPARATION-STATUS.json`, the payload README and the source manifests before publishing. Rebuild/validate the native third-party variants from preserved sources, inspect the four metadata-only Rust license declarations, check supported-machine operation, and confirm distribution terms/artwork rights. Do not mark this review complete merely because the scripts succeed.

After review, create a **new** public distribution repository, copy README/Casks/notices and source result manifests only, and upload all installer/source archives plus SHA256SUMS to `v0.1.0-preview.2`. Do not push the easycut Git history, Cargo workspace, local credentials, staging app directory or working files. Publish source assets beside the binaries at the same release/download location, without authentication or fees, and retain them for as long as corresponding binaries are offered. GPLv3 section 6(d) permits an equivalent source download alongside object-code downloads; an upstream URL or incomplete source recipe list alone is not the prepared distribution method.

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

Five Python regression tests passed, along with cargo fmt, Clippy with warnings denied, and all 58 Rust unit/integration tests. The prepared Windows/Linux archives preserve the original application/tool executable bytes; the macOS copied app preserves those executable bytes and passes strict recursive signature verification. The new DMG passes hdiutil verification and its generated Cask passes Homebrew style checks. The public source indexes exclude workspace packages and the source archive paths exclude easycat workspace directories and Git history. Native third-party source reconstruction and legal release review have not been completed.
