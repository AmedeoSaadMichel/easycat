#!/usr/bin/env python3
"""Prepare an allowlisted public release directory without easycat source/history."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import zipfile

from third_party_notices import prepare_notices

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ROOT / 'dist/third-party-sources'
ORIGINALS = {
    'easycat-0.1.0-macos-arm64.dmg': '57d91d9a8888b456235214df72895b251861de57ce5655483b2ed5c539befec4',
    'easycat-0.1.0-windows-x64.zip': 'aed0e92984ae15198f060473c46dc742d59aa999762082b6ba4fafd0a47190be',
    'easycat-0.1.0-linux-x86_64.tar.gz': 'c622fa35edff134c57b08f1f10c4edb598131ebdea25cd879d205d49ca5f4ead',
}


def digest(path):
    with path.open('rb') as stream:
        checksum = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            checksum.update(chunk)
        return checksum.hexdigest()


def bundle_sources(path, files, extra_directory=None):
    with tarfile.open(path, 'w:gz', compresslevel=1) as archive:
        for file in sorted(set(files)):
            archive.add(file, arcname='third-party/' + file.name)
        public_files = json.loads((ROOT / 'distribution/public-files.json').read_text())
        for relative in public_files:
            source = ROOT / 'distribution' / relative
            if source.is_symlink() or not source.resolve().is_relative_to((ROOT / 'distribution').resolve()):
                raise RuntimeError('Unsafe public notice path')
            archive.add(source, arcname='compliance-and-notices/' + relative)
        if extra_directory:
            archive.add(extra_directory, arcname='homebrew-recipes-and-notices')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', required=True, help='Future public OWNER/REPO; no repository is created')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist/public-distribution')
    parser.add_argument('--resume', action='store_true', help='Resume this generated review directory after an interrupted packaging run')
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', args.repository):
        parser.error('Repository must be OWNER/REPO')
    if args.repository.lower() == 'amedeosaadmichel/easycut':
        parser.error('Use a separate public distribution repository; keep easycut private')
    if digest(ROOT / 'Cargo.lock') != 'd6d8c3585a1c883c260871d0627316ae607822a68c7dcf2457fe3307fcb69d87':
        parser.error('The original preview dependency lock changed; re-audit the actual binary dependencies')
    mac = json.loads((SOURCES / 'macos-source-results.json').read_text())
    portable = json.loads((SOURCES / 'portable-source-results.json').read_text())
    rust = json.loads((SOURCES / 'ffmpeg-rust-vendor-results.json').read_text())
    for records, expected in [(mac, 'verified'), (portable, 'collected'), (rust, 'collected')]:
        if any(record['status'] != expected for record in records):
            parser.error('Some corresponding source trees are incomplete')
        for record in records:
            expected_hash = record.get('sha256_collected', record.get('sha256'))
            if digest(SOURCES / record['filename']) != expected_hash:
                parser.error('Collected source checksum mismatch: ' + record['filename'])
    upstream = json.loads((ROOT / 'distribution/compliance/upstream-source-artifacts.json').read_text())
    for record in upstream:
        if digest(SOURCES / record['filename']) != record['sha256']:
            parser.error('Upstream source hash mismatch: ' + record['filename'])
    for name, checksum in ORIGINALS.items():
        if digest(ROOT / 'dist' / name) != checksum:
            parser.error('Original package checksum mismatch: ' + name)
    output = args.output.resolve()
    if args.resume and not (output / 'macos-source-results.json').is_file():
        parser.error('Resume requires an existing generated review directory')
    output.mkdir(parents=True, exist_ok=args.resume)
    notices = output / 'notices'
    if not (args.resume and notices.is_dir()):
        metadata = json.loads(subprocess.check_output(
            ['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], cwd=ROOT, text=True))
        prepare_notices(notices, metadata)
    for file in notices.rglob('*'):
        if file.is_file():
            os.utime(file, (315532800, 315532800))
    # Explicit inputs only: no repository glob, Git history, workspace metadata or Rust app sources.
    for name in ('macos-source-results.json', 'portable-source-results.json',
                 'ffmpeg-rust-vendor-results.json', 'rust-third-party-index.json'):
        shutil.copy2(SOURCES / name, output / name)
    if not (args.resume and (output / 'easycat-0.1.0-macos-third-party-source.tar.gz').exists()):
        bundle_sources(output / 'easycat-0.1.0-macos-third-party-source.tar.gz',
                       [SOURCES / record['filename'] for record in mac],
                       ROOT / 'target/macos-package/easycat.app/Contents/Resources/Licenses')
    files = [SOURCES / record['filename'] for record in portable + rust]
    files += [SOURCES / 'ffmpeg-669f5ab083c7732c10f1ce9e4f1915960dd88457.tar.gz',
              SOURCES / 'btbn-build-recipes-63ac3265b0789ea16c7b1f8351cd2ad195a28722.tar.gz']
    if not (args.resume and (output / 'easycat-0.1.0-portable-third-party-source.tar.gz').exists()):
        bundle_sources(output / 'easycat-0.1.0-portable-third-party-source.tar.gz', files)
    shutil.copy2(SOURCES / 'rust-third-party-sources.tar.gz', output)
    original = ROOT / 'dist/easycat-0.1.0-windows-x64.zip'
    windows = output / 'easycat-0.1.0-windows-x64-legal-preview.zip'
    with zipfile.ZipFile(original) as source, zipfile.ZipFile(windows, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        prefix = source.namelist()[0].split('/')[0]
        for member in source.infolist():
            archive.writestr(member, source.read(member))
        for file in notices.rglob('*'):
            if file.is_file():
                archive.write(file, prefix + '/Licenses/ThirdParty/' + str(file.relative_to(notices)))
    original = ROOT / 'dist/easycat-0.1.0-linux-x86_64.tar.gz'
    linux = output / 'easycat-0.1.0-linux-x86_64-legal-preview.tar.gz'
    with tarfile.open(original) as source, tarfile.open(linux, 'w:gz', compresslevel=6) as archive:
        prefix = source.getnames()[0].split('/')[0]
        for member in source:
            archive.addfile(member, source.extractfile(member) if member.isfile() else None)
        archive.add(notices, arcname=prefix + '/Licenses/ThirdParty')
    stage = output / 'macos-image'
    if args.resume and stage.exists():
        shutil.rmtree(stage)
    stage.mkdir()
    app = stage / 'easycat.app'
    shutil.copytree(ROOT / 'target/macos-package/easycat.app', app, symlinks=True)
    shutil.copytree(notices, app / 'Contents/Resources/Licenses/ThirdParty')
    subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    (stage / 'Applications').symlink_to('/Applications')
    mac_dmg = output / 'easycat-0.1.0-macos-arm64-legal-preview.dmg'
    if args.resume and mac_dmg.exists():
        mac_dmg.unlink()
    subprocess.run(['hdiutil', 'create', '-volname', 'easycat', '-srcfolder', str(stage), '-format', 'UDZO', str(mac_dmg)], check=True)
    subprocess.run(['hdiutil', 'verify', str(mac_dmg)], check=True)
    assets = [file for file in output.iterdir() if file.name.endswith(('.tar.gz', '.zip', '.dmg'))]
    checksums = {file.name: digest(file) for file in assets}
    (output / 'SHA256SUMS').write_text(''.join(f'{checksum}  {name}\n' for name, checksum in sorted(checksums.items())))
    cask = output / 'Casks/easycat.rb'
    cask.parent.mkdir()
    text = (ROOT / 'Casks/easycat.rb').read_text()
    text = text.replace('0.1.0,preview.1', '0.1.0,preview.2').replace(ORIGINALS['easycat-0.1.0-macos-arm64.dmg'], checksums[mac_dmg.name])
    text = text.replace('AmedeoSaadMichel/easycut', args.repository).replace('-macos-arm64.dmg', '-macos-arm64-legal-preview.dmg')
    cask.write_text(text)
    (output / 'PREPARATION-STATUS.json').write_text(json.dumps({
        'repository': args.repository, 'published': False, 'owner_release_review_required': True,
        'source_trees_collected': {'macos': len(mac), 'portable_candidates': len(portable)},
        'original_application_binaries_changed': False, 'macos_signature': 'ad-hoc, not notarized',
        'source_repo_must_remain_private': True, 'asset_sha256': checksums,
    }, indent=2) + '\n')
    base = f'https://github.com/{args.repository}'
    release = base + '/releases/download/v0.1.0-preview.2/'
    (output / 'README.md').write_text(f'''# easycat downloads

Cut. Transform. Meow. Local video trimming, compression and container conversion.
This repository distributes binaries and third-party source materials only. The easycat application source and development history remain private.

## Install

- [macOS Apple Silicon DMG]({release}{mac_dmg.name}) — macOS 15 or later; ad-hoc signed preview, not notarized.
- [Windows x64 ZIP]({release}{windows.name}) — extract the entire package and run easycat.exe; unsigned preview.
- [Linux x86_64 tar.gz]({release}{linux.name}) — glibc 2.35+, extract and run ./easycat.

On macOS, add the personal tap once:

```sh
brew tap AmedeoSaadMichel/easycat {base}.git
brew install --cask easycat
```

An existing easycat tap pointing to the private development repository must be untapped and added again with the public URL. Homebrew does not remove macOS signature limitations; use normal Privacy & Security controls if macOS asks for approval.

## Third-party software and source

These packages contain FFmpeg under GPLv3-or-later, including GPL codec components. The easycat app runs FFmpeg as a separate executable. Third-party license rights remain intact.

- [macOS dependency source]({release}easycat-0.1.0-macos-third-party-source.tar.gz)
- [Windows/Linux dependency source]({release}easycat-0.1.0-portable-third-party-source.tar.gz)
- [Rust third-party crate source]({release}rust-third-party-sources.tar.gz)
- [Checksums]({release}SHA256SUMS)

Licenses and notices are included in each installer under Licenses/ThirdParty (inside easycat.app/Contents/Resources on macOS). Preserve the source archives beside each released binary indefinitely. Publishing these materials does not publish easycat's application source.

This directory is prepared for review; the repository and release must exist before these links work.
''')
    print('Prepared review directory:', output)
    print('No repository was created or published. Read docs/private-code-distribution.md before uploading.')


if __name__ == '__main__':
    main()
