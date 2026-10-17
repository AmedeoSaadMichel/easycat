#!/usr/bin/env python3
"""Assemble evidence-bound private-source distribution readiness from completed checks."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load(path):
    return json.loads(path.read_text())


def require_passed(report):
    if report.get('status') != 'passed':
        raise ValueError('A required native check has not passed')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--linux', type=Path, required=True)
    parser.add_argument('--windows', type=Path, required=True)
    args = parser.parse_args()
    cases = load(ROOT / 'distribution/resolved-crate-licenses/resolutions.json')
    if len(cases) != 4 or any(case['status'] != 'resolved' for case in cases):
        raise ValueError('Unresolved Rust license cases')
    for case in cases:
        folder = ROOT / 'distribution/resolved-crate-licenses' / case['crate']
        if hashlib.sha256((folder / 'published-Cargo.toml').read_bytes()).hexdigest() != case['published_manifest_sha256']:
            raise ValueError('License evidence checksum mismatch')
        if '<year>' in (ROOT / 'distribution/resolved-crate-licenses' / case['notice']).read_text():
            raise ValueError('Unresolved copyright placeholder')
    native = {}
    for name, directory in [('linux', args.linux), ('windows', args.windows)]:
        core = load(directory / 'source-build-check/result.json')
        runtime = load(directory / 'native-package-tests/result.json')
        require_passed(core); require_passed(runtime)
        if core['source_sha256'] != 'e6cfdd7b8c708239dcc39a71fc454c44b8c95de7b74d014c2f4ba4e06a6a0c52':
            raise ValueError('Unexpected portable FFmpeg source')
        core['configuration'] = ['--prefix=<verification-prefix>' if flag.startswith('--prefix=') else flag for flag in core['configuration']]
        native[name] = {'source_build': core, 'distributed_tools': runtime}
    mac_build = load(ROOT / 'target/license-audit/native-rebuild/macos/result.json')
    mac_runtime = load(ROOT / 'target/license-audit/native-tests/macos/result.json')
    mac_rebuilt = load(ROOT / 'target/license-audit/native-tests/macos-rebuilt/result.json')
    require_passed(mac_runtime); require_passed(mac_rebuilt)
    if mac_build['status'] != 'built' or mac_build['source_sha256'] != '8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e':
        raise ValueError('macOS source reconstruction not verified')
    mac_build['configuration'] = ['--prefix=<verification-prefix>' if flag.startswith('--prefix=') else flag for flag in mac_build['configuration']]
    native['macos'] = {'source_build': mac_build, 'distributed_tools': mac_runtime, 'rebuilt_tools': mac_rebuilt}
    linkage = load(ROOT / 'distribution/compliance/app-linkage-audit.json')
    if any(item['direct_ffmpeg_library_linkage'] for item in linkage.values()):
        raise ValueError('Direct GPL linking needs a new licensing review')
    report = {'status': 'passed', 'ready_for_public_preview': True, 'private_app_source_included': False,
              'license_cases_resolved': [case['crate'] for case in cases], 'native_verification': native,
              'linkage': linkage,
              'source_completeness_method': 'Pinned source archives, original recipes/patches, complete submodules and locked Rust vendor sources',
              'full_dependency_toolchain_reproduction_claimed': False,
              'scope': 'Technical/source-license preview release checks. Windows/Linux rebuild scope is the FFmpeg core; actual distributed external-codec configurations are runtime tested. All upstream dependency binaries have not been rebuilt byte-for-byte.',
              'remaining_preview_limits': ['macOS ad-hoc signature without notarization', 'Windows unsigned binaries', 'Interactive desktop and GPU behaviour are not certified by headless checks']}
    (ROOT / 'distribution/compliance/release-verification.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Evidence-bound public preview verification report generated')


if __name__ == '__main__':
    main()
