#!/usr/bin/env python3
"""Build a self-contained, ad-hoc signed macOS disk image from local binaries."""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import platform
import re
import shutil
import subprocess

from third_party_notices import prepare_notices

ROOT = Path(__file__).resolve().parents[1]
VERSION = '0.1.0'
OUT = ROOT / 'dist'
STAGE = ROOT / 'target' / 'macos-package'
APP = STAGE / 'easycat.app'
CONTENTS = APP / 'Contents'
FRAMEWORKS = CONTENTS / 'Frameworks'
RESOURCES = CONTENTS / 'Resources'
HELPERS = CONTENTS / 'Helpers'
MACOS = CONTENTS / 'MacOS'


def run(*args):
    try:
        return subprocess.check_output([str(arg) for arg in args], text=True, stderr=subprocess.STDOUT)
    except subprocess.CalledProcessError as error:
        raise RuntimeError(error.output) from error


def dependencies(path):
    return [line.strip().split(' (compatibility')[0] for line in run('otool', '-L', path).splitlines()[1:] if line.strip()]


def system(path):
    return path.startswith(('/System/', '/usr/lib/'))


def package_architecture(machine=None):
    machine = machine or platform.machine()
    if machine not in ('arm64', 'x86_64'):
        raise RuntimeError(f'Unsupported macOS architecture: {machine}')
    return machine


def check_binary_architecture(path, architecture):
    if architecture not in run('lipo', '-archs', path).split():
        raise RuntimeError(f'Wrong architecture in {path}; expected {architecture}')


def main():
    architecture = package_architecture()
    if APP.exists():
        raise SystemExit(f'Remove or rename the previous generated staging bundle first: {APP}')
    for directory in [FRAMEWORKS, RESOURCES, HELPERS, MACOS, OUT]:
        directory.mkdir(parents=True, exist_ok=True)
    binary = ROOT / 'target/release/easycat-app'
    shutil.copy2(binary, MACOS / 'easycat-bin')
    queue = [(binary.resolve(), MACOS / 'easycat-bin')]
    for name in ['ffmpeg', 'ffprobe', 'ffplay']:
        source = Path(shutil.which(name) or name).resolve(strict=True)
        dest = HELPERS / name
        shutil.copy2(source, dest)
        queue.append((source, dest))
    # SDL2 compatibility loads SDL3 dynamically rather than through a Mach-O link.
    brew_prefix = Path(run('brew', '--prefix').strip())
    sdl3 = brew_prefix / 'opt/sdl3/lib/libSDL3.0.dylib'
    if sdl3.exists():
        queue.append((sdl3.resolve(), FRAMEWORKS / sdl3.name))
        shutil.copy2(sdl3, FRAMEWORKS / sdl3.name)
    known = {}
    packages = set()
    manifest = []
    minimum = (11, 0)
    for source, dest in queue:
        if dest.name in known:
            if known[dest.name] != source:
                raise RuntimeError(f'Conflicting library names: {source}, {known[dest.name]}')
            continue
        known[dest.name] = source
        check_binary_architecture(source, architecture)
        dest.chmod(0o755)
        build = run('otool', '-l', source)
        for version in re.findall(r'\bminos\s+(\d+(?:\.\d+)+)', build):
            minimum = max(minimum, tuple(map(int, version.split('.'))))
        for dep in dependencies(source):
            if system(dep):
                continue
            if not dep.startswith('/'):
                raise RuntimeError(f'Unresolved non-absolute dependency {dep} in {source}')
            original = Path(dep).resolve(strict=True)
            if original == source:
                run('install_name_tool', '-id', '@loader_path/' + dest.name, dest)
                continue
            target = FRAMEWORKS / original.name
            if not target.exists():
                shutil.copy2(original, target)
            queue.append((original, target))
            relative = os.path.relpath(target, dest.parent)
            run('install_name_tool', '-change', dep, '@loader_path/' + relative, dest)
        if '/Cellar/' in str(source):
            cellar, suffix = str(source).split('/Cellar/', 1)
            parts = suffix.split('/')
            packages.add(Path(cellar) / 'Cellar' / parts[0] / parts[1])
        manifest.append({'source': str(source), 'bundled': str(dest.relative_to(APP)), 'sha256_original': hashlib.sha256(source.read_bytes()).hexdigest()})
    licenses = RESOURCES / 'Licenses'
    licenses.mkdir()
    prepare_notices(licenses / 'ThirdParty')
    shutil.copy2(ROOT / 'LICENSE', licenses / 'easycat-MIT.txt')
    for package in sorted(packages):
        destination = licenses / (package.parent.name + '-' + package.name)
        destination.mkdir()
        for item in package.iterdir():
            if item.is_file() and (item.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE', 'COPYRIGHT', 'AUTHORS')) or item.name in ['INSTALL_RECEIPT.json', 'sbom.spdx.json']):
                shutil.copy2(item, destination / item.name)
        formula = package / '.brew'
        if formula.exists():
            shutil.copytree(formula, destination / 'formula')
    for item in (ROOT / 'easycat-app/assets').glob('*OFL.txt'):
        shutil.copy2(item, licenses / item.name)
    update_source = (ROOT / 'easycat-app/src/updates.rs').read_text()
    build = int(re.search(r'pub const BUILD: u64 = (\d+);', update_source).group(1))
    app_version = re.search(r'pub const VERSION: &str = "([^"]+)";', update_source).group(1)
    (RESOURCES / 'build-info.json').write_text(json.dumps({
        'architecture': architecture, 'application_build': build, 'application_version': app_version,
        'source_commit': run('git', 'rev-parse', 'HEAD').strip(),
        'workflow_run': os.environ.get('GITHUB_RUN_ID'),
        'homebrew_components': sorted(package.parent.name + '-' + package.name for package in packages),
    }, indent=2) + '\n')
    (RESOURCES / 'dependency-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    if (FRAMEWORKS / 'libSDL3.0.dylib').exists():
        (FRAMEWORKS / 'libSDL3.dylib').symlink_to('libSDL3.0.dylib')
    launcher = MACOS / 'easycat'
    launcher.write_text('''#!/bin/sh
contents="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export PATH="$contents/Helpers:/usr/bin:/bin:/usr/sbin:/sbin"
exec "$contents/MacOS/easycat-bin" "$@"
''')
    launcher.chmod(0o755)
    iconset = STAGE.parent / 'easycat.iconset'
    iconset.mkdir(exist_ok=True)
    icon_source = ROOT / 'easycat-app/assets/app-icon.png'
    for size in [16, 32, 128, 256, 512]:
        for density in [1, 2]:
            suffix = '@2x' if density == 2 else ''
            run('sips', '-z', size * density, size * density, icon_source,
                '--out', iconset / f'icon_{size}x{size}{suffix}.png')
    run('iconutil', '-c', 'icns', iconset, '-o', RESOURCES / 'easycat.icns')
    info = {'CFBundleName': 'easycat', 'CFBundleDisplayName': 'easycat', 'CFBundleIdentifier': 'com.easycat.desktop', 'CFBundleExecutable': 'easycat', 'CFBundleIconFile': 'easycat.icns', 'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': VERSION, 'CFBundleVersion': str(build), 'LSMinimumSystemVersion': '.'.join(map(str, minimum)), 'NSHighResolutionCapable': True, 'NSPrincipalClass': 'NSApplication', 'NSSupportsAutomaticGraphicsSwitching': True}
    with (CONTENTS / 'Info.plist').open('wb') as stream:
        plistlib.dump(info, stream)
    for _, dest in queue:
        for dep in dependencies(dest):
            if not system(dep) and not dep.startswith('@loader_path/'):
                raise RuntimeError(f'External dependency remains in {dest}: {dep}')
    for dest in sorted(set(dest for _, dest in queue)):
        run('codesign', '--force', '--sign', '-', dest)
    run('codesign', '--force', '--sign', '-', APP)
    run('codesign', '--verify', '--deep', '--strict', APP)
    (STAGE / 'Applications').symlink_to('/Applications')
    (STAGE / 'Read me.txt').write_text(f'''easycat {app_version} — {'Apple Silicon' if architecture == 'arm64' else 'Intel x86_64'}

Drag easycat to Applications to install.
Requires macOS {info['LSMinimumSystemVersion']} or later.
FFmpeg, ffprobe, ffplay and their linked libraries are included.
This local test build is ad-hoc signed, without Developer ID notarization.
Third-party licenses and build manifests are inside easycat.app/Contents/Resources.
This image is a local test artifact; public release compliance and notarization
have not been completed.
''')
    dmg = OUT / f'easycat-{VERSION}-macos-{architecture}.dmg'
    if dmg.exists():
        raise RuntimeError(f'Output already exists: {dmg}')
    run('hdiutil', 'create', '-volname', 'easycat', '-srcfolder', STAGE, '-ov', '-format', 'UDZO', dmg)
    run('hdiutil', 'verify', dmg)
    checksum = hashlib.sha256(dmg.read_bytes()).hexdigest()
    dmg.with_suffix('.dmg.sha256').write_text(f'{checksum}  {dmg.name}\n')
    print(json.dumps({'dmg': str(dmg), 'bytes': dmg.stat().st_size, 'architecture': architecture, 'application_build': build, 'minimum_macos': info['LSMinimumSystemVersion'], 'libraries': len(known), 'sha256': checksum}, indent=2))


if __name__ == '__main__':
    main()
