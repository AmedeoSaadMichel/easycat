# Update notifications and minimum supported builds

Update-aware easycat builds check the public policy once at startup, off the UI thread. The footer **Updates** button checks again when an update is available or a check has failed. It stays hidden while checking and when the app is current. Downloads open the release page in the external browser; the app never installs or replaces itself automatically.

The endpoint is `https://raw.githubusercontent.com/AmedeoSaadMichel/easycat-releases/main/updates.json`. The maintained copy is `distribution/updates.json`. The update policy is hosted separately from the MIT-licensed [application source repository](https://github.com/AmedeoSaadMichel/easycat).

## Policy

`schema_version` is 1. Each supported platform (`macos-arm64`, `windows-x64`, `linux-x86_64`) has:

- `latest_build`: newest published application build on that platform.
- `minimum_build`: lowest build allowed after a successful check.
- `version`: published release version, for example `0.1.0-preview.3`; the app constructs the link inside the fixed easycat release repository.

Builds below `minimum_build` show a mandatory update screen. They cannot open videos or start new exports. Playback and preset measurement stop when the gate becomes active. An already-running export can finish or be cancelled normally before the gate applies. Builds at or above the minimum but below `latest_build` show a dismissible update notice.

Connection errors, timeouts, HTTP errors, unsupported platforms, missing HTTPS clients and invalid manifests never lock the app. **Check again** retries a failed check. A failed retry also clears a previously received block for the current session; a pending retry keeps that block until its outcome is known. No minimum policy is persisted across launches. Consequently this is an operational update policy, not offline DRM or protection against deliberately disabling connectivity.

## Publish a new build

1. Increment `BUILD` and update `VERSION` in `easycat-app/src/updates.rs` before compiling all platform packages. Build 4 is the first published update-aware release.
2. Publish and verify the actual installers, corresponding third-party sources and Homebrew cask first.
3. Update the relevant platform's `latest_build` and `version` in `distribution/updates.json`. Do not advertise a version before its release downloads work. Platforms can advance independently.
4. Raise `minimum_build` only when older update-aware builds must be retired. Keep it positive and no greater than `latest_build`.
5. Copy the reviewed JSON to the root `updates.json` of the public `easycat-releases` repository and publish that file. To roll back a requirement, lower the minimum and check again in the app.

The live policy advertises preview 6 with latest build 6 and minimum build 4 on all three platforms. Preview 6 is optional for preview 4 and 5 users. It was activated only after anonymous download and checksum verification. Preview 3 and earlier do not contain the checker and cannot be retroactively blocked. Users of preview 3 or earlier must install a current preview once to receive future notifications. A policy-only change does not update an installed binary.


## Transport and privacy

The checker uses the system `curl` executable over HTTPS: `/usr/bin/curl` on macOS/Linux and `System32/curl.exe` on Windows. If unavailable, the check fails and local use continues. Linux users need curl installed to receive notifications. No new networking library or bundled executable is introduced. The existing locked serde/serde_json packages are declared directly by the app; third-party package versions and source materials are unchanged.

Requests contain no video paths, media, account identifiers or usage events. GitHub receives normal connection metadata, including the request IP. Local curl configuration is disabled. Connections and responses have time/size limits, TLS verification remains enabled, and redirects are not followed. The UI stays responsive while the request runs. Tests use injected responses and never contact the network.

## Local simulations

Development builds accept `--simulate-update=optional` or `--simulate-update=required`. Each opens a clearly marked local simulation of the next build against the current build. Retry remains local and the download button is disabled; public policy is never changed. These flags and their state overrides are excluded from release builds.

```sh
cargo run -p easycat-app -- --simulate-update=optional
cargo run -p easycat-app -- --simulate-update=required
```
