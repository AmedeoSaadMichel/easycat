# easycat — project instructions

> Rules are ordered by priority. When they conflict, the higher rule wins.
> "STOP" means request explicit confirmation. "NEVER" is non-negotiable.

## Product boundaries

**easycat** is a desktop app for trimming, compressing, resizing and changing video containers efficiently. Efficient means fast, low energy use, and avoiding unnecessary quality loss.

- Local, private and free: no cloud, uploads or accounts.
- Native macOS, Windows and Linux app. Mobile and web are outside the first release.
- Do not add networking, telemetry, analytics, login, advertising or automatic updates without explicit confirmation.
- Never modify users' source videos in place. Output must be a new file.

## Stop-the-line actions

Do not perform these actions without explicit user confirmation:

- **Git:** commit, push, reset --hard, rebase, or any command changing history or remotes. Propose the command and wait for authorization.
- **AI attribution:** NEVER add AI names or attribution to commit messages, PRs or Git metadata. No AI Co-Authored-By or Generated with trailers; describe the change itself.
- **Dependencies:** adding, removing or updating crates in Cargo.toml.
- **Architecture:** changing boundaries between easycat-core, easycat-cli and easycat-app.
- **FFmpeg pipeline:** changing command construction, encoder selection, stream copy or process management.
- **Licensing/distribution:** bundling FFmpeg, x264/x265 or other codecs, including GPL and patent considerations.
- **Privacy/scope:** networking, uploads, analytics or synchronization.
- **Manual GUI launch:** do not run cargo run -p easycat-app without an explicit request. Build, check, clippy, test and CLI runs on generated clips are authorized.
- **User files:** NEVER delete or overwrite users' videos. Test only with generated clips or copies.

Changes limited to egui layout/style do not require confirmation when they do not change core calls. Existing explicit user authorization takes precedence within its scope.

## Language

Use **English** for the interface, CLI help, errors, comments, documentation, tests and proposed commit messages. This is the user's current project language requirement. Conversation may follow the user's language. Standard technical terms remain unchanged.

## Stack

- Rust for both engine and interface.
- egui/eframe for the native UI; check compatibility with eframe 0.29.
- External FFmpeg processes for codecs. Future libav bindings must remain behind the existing engine boundary.
- Tauri was rejected; do not revisit it without a new decision.

## Architecture

The workspace uses the easycat name for all crates:

```text
easycat-core/ -> engine: probing, trimming, transcoding, encoder selection; no UI dependencies
easycat-cli/  -> thin CLI calling the core
easycat-app/  -> thin native egui UI calling the core
```

### UI-independent core

- CLI and GUI call the same easycat-core functions; keep business logic out of both frontends.
- Core does not depend on egui or clap.

### FFmpeg boundary

- FFmpeg invocation knowledge belongs in the core's process boundary; other modules use typed jobs/settings rather than argument strings.
- NEVER concatenate shell commands from user input. Pass argument lists directly, without a shell.

### Errors

- Return Result with core error types.
- No unwrap/expect outside tests unless documenting a genuine invariant.
- Do not let panics reach the UI; show readable errors.
- Capture FFmpeg stderr and include it in failures.

### Long operations

- Never block the UI thread. Use workers with progress and cancellation.
- Failed or cancelled exports must not leave partial files.

### Single source of truth

- Keep encoder selection and profile definitions centralized. Report duplication; do not merge independent implementations without confirmation.

## Efficiency principles

1. Use stream copy for compatible trimming: fast and lossless, with keyframe-only precision. Explain this limitation in the UI.
2. Prefer hardware encoding where supported: VideoToolbox on macOS; NVENC/QSV/AMF are future Windows/Linux possibilities. Fall back to software when hardware fails.
3. Future precise trimming should re-encode only the boundary frames and copy the interior.
4. Avoid unnecessary transcoding when the requested operation can genuinely use compatible stream copy; do not bypass requested compression simply because codec and dimensions match.
5. Quality comes before size; do not silently degrade video in defaults.

## Current state and next work

The workspace builds and has generated-clip integration coverage. The UI includes playback, thumbnails, trim handles, profiles, stream-copy container conversion, progress and cancellation. The public app name is easycat.

Remaining areas include precise boundary-only re-encoding, additional platform hardware backends, queues and release packaging with FFmpeg. Treat old benchmark conclusions as machine-specific measurements. VAAPI is not implemented. Evaluate codec licenses and patents before distributing bundles.

## File size

- Aim for at most 500 lines per file.
- 501–800 lines is acceptable.
- Above 800 lines requires refactoring or an explicit exception.
- Split responsibilities rather than introducing arbitrary abstractions.

## Rust file headers

Every created or modified Rust file starts with its workspace-relative path:

```rust
// File: easycat-core/src/path/to/file.rs
```

## Test-driven development and checks

- Use TDD where it is meaningful: describe expected behavior in a failing test, implement the minimum, then refactor with green tests.
- Add regression coverage for bugs.
- Core argument construction should be testable without FFmpeg; integration tests use short generated clips, never user videos.
- If a test-first approach is impractical for exploratory UI or hardware behavior, explain why and use the nearest meaningful verification.
- Before finishing: cargo fmt, cargo clippy --workspace -- -D warnings, cargo test --workspace.

## UI

- Keep the interface clear across viewport sizes and display scales; avoid truncated text and unreachable controls.
- Always expose source, requested operation, progress, cancellation and output path.
- Follow the approved dark cosmic easycat design. Preserve readability and native interaction.

## Working practices

### Think before editing

- State assumptions before implementation.
- If interpretations differ materially, describe the options rather than silently choosing.
- Propose a simpler approach when one exists.
- Ask when ambiguity blocks safe progress.

### Simplicity

- Write the minimum code that solves the requested problem.
- Avoid speculative features, one-off abstractions and unnecessary configuration.
- Prefer a small implementation over an unnecessarily large one.

### Surgical changes

- Change only what the request requires.
- Do not clean up adjacent style or formatting without a reason.
- Do not refactor working code gratuitously.
- Remove orphans introduced by your changes; report pre-existing dead code instead of deleting it.

### Proceed versus ask

- Layout/style only: proceed.
- Local typo fixes or narrowly scoped renames: proceed.
- FFmpeg pipeline, encoder selection or process management: require authorization.
- New crates or workspace-boundary changes: require authorization.
- Licensing, packaging and distribution: require authorization.
- Nontrivial trade-offs or blocking ambiguity: explain the choices and ask when existing authorization does not resolve them.
