#!/usr/bin/env python3
"""Package native Windows/Linux release binaries with pinned FFmpeg tools."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / 'target'
VERSION = '0.1.0'
CONFIG = json.loads((ROOT / 'scripts/ffmpeg-bundles.json').read_text())


def run(args, **kwargs):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, timeout=180, **kwargs)


def vendor_dir():
    candidates = list((TARGET / 'ffmpeg').glob('*/bin'))
    if len(candidates) != 1:
        raise RuntimeError(f'Expected one extracted tool directory: {candidates}')
    return candidates[0].parent


def prepare(platform):
    config = CONFIG[platform]
    download = TARGET / config['url'].rsplit('/', 1)[1]
    TARGET.mkdir(exist_ok=True)
    urllib.request.urlretrieve(config['url'], download)
    if hashlib.sha256(download.read_bytes()).hexdigest() != config['sha256']:
        raise RuntimeError('FFmpeg archive checksum mismatch')
    directory = TARGET / 'ffmpeg'
    directory.mkdir()
    if platform == 'windows':
        with zipfile.ZipFile(download) as archive:
            if any(Path(name).is_absolute() or '..' in Path(name).parts for name in archive.namelist()):
                raise RuntimeError('Unsafe archive member')
            archive.extractall(directory)
    else:
        with tarfile.open(download) as archive:
            archive.extractall(directory, filter='data')
    vendor = vendor_dir()
    suffix = '.exe' if platform == 'windows' else ''
    for name in ['ffmpeg', 'ffprobe', 'ffplay']:
        if not (vendor / 'bin' / (name + suffix)).is_file():
            raise RuntimeError(f'Missing bundled executable: {name}')
    if os.environ.get('GITHUB_PATH'):
        with open(os.environ['GITHUB_PATH'], 'a') as output:
            output.write(str(vendor / 'bin') + '\n')
        if platform == 'linux':
            with open(os.environ['GITHUB_ENV'], 'a') as output:
                output.write(f'LD_LIBRARY_PATH={vendor / "lib"}\n')
    print('FFmpeg prepared:', vendor, flush=True)


def package(platform):
    suffix = '.exe' if platform == 'windows' else ''
    name = f'easycat-{VERSION}-' + ('windows-x64' if platform == 'windows' else 'linux-x86_64')
    stage = TARGET / name
    stage.mkdir()
    helpers = stage / 'Helpers'
    vendor = vendor_dir()
    shutil.copytree(vendor / 'bin', helpers)
    if (vendor / 'lib').exists():
        shutil.copytree(vendor / 'lib', stage / 'lib', symlinks=True)
    binary = stage / ('easycat.exe' if platform == 'windows' else 'easycat-bin')
    shutil.copy2(TARGET / 'release' / ('easycut-app' + suffix), binary)
    shutil.copy2(ROOT / 'easycut-app/assets/app-icon.png', stage / 'easycat.png')
    licenses = stage / 'Licenses'
    licenses.mkdir()
    for item in vendor.iterdir():
        if item.is_file():
            shutil.copy2(item, licenses / item.name)
        elif item.name in ['doc', 'licenses', 'LICENSES']:
            shutil.copytree(item, licenses / item.name)
    for item in (ROOT / 'easycut-app/assets').glob('*OFL.txt'):
        shutil.copy2(item, licenses / item.name)
    shutil.copy2(ROOT / 'scripts/ffmpeg-bundles.json', licenses / 'ffmpeg-source.json')
    if platform == 'linux':
        launcher = stage / 'easycat'
        launcher.write_text('''#!/bin/sh
app="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
export PATH="$app/Helpers:$PATH"
export LD_LIBRARY_PATH="$app/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$app/easycat-bin" "$@"
''')
        launcher.chmod(0o755)
        (stage / 'easycat.desktop').write_text('''[Desktop Entry]
Type=Application
Name=easycat
Comment=Cut. Transform. Meow.
Exec=easycat
Icon=easycat
Terminal=false
Categories=AudioVideo;Video;
''')
    (stage / 'Read me.txt').write_text(f'''easycat {VERSION} — {platform} x64 preview

Extract the entire archive before running the app.
Windows: double-click easycat.exe. Keep Helpers beside the executable.
Linux: run ./easycat from the extracted directory. Keep lib and Helpers in place.
FFmpeg, ffprobe and ffplay are included. Rust is not required.
Windows 10/11 x64 or Linux x86_64 with glibc 2.35+ (Ubuntu 22.04+).
Linux needs an X11/Wayland desktop, an OpenGL-capable driver and xdg-desktop-portal.
This preview is not code-signed. OS security prompts may be shown.
Software encoding is used on Windows/Linux; GPU export is currently macOS-only.
Third-party licenses and pinned FFmpeg source/build metadata are in Licenses.
Public distribution compliance review remains pending for this test preview.
''')
    env = os.environ.copy()
    env['PATH'] = str(helpers) + os.pathsep + env.get('PATH', '')
    if platform == 'linux':
        env['LD_LIBRARY_PATH'] = str(stage / 'lib')
    env.update(SDL_AUDIODRIVER='dummy', SDL_VIDEODRIVER='dummy')
    smoke = TARGET / ('portable-smoke-' + platform)
    smoke.mkdir()
    source = smoke / 'source.mp4'
    run([helpers / ('ffmpeg' + suffix), '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=160x120:rate=10', '-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo', '-t', '1', '-c:v', 'libx264', '-c:a', 'aac', source], env=env)
    run([helpers / ('ffprobe' + suffix), '-v', 'error', '-show_entries', 'format=duration', source], env=env)
    cli = TARGET / 'release' / ('easycut' + suffix)
    run([cli, 'cut', source, smoke / 'trim.mp4', '--start', '0', '--end', '0.5'], env=env)
    run([cli, 'remux', source, smoke / 'remux.mkv'], env=env)
    run([cli, 'compress', source, smoke / 'compressed.mp4', '--software', '--height', '96'], env=env)
    run([helpers / ('ffplay' + suffix), '-nodisp', '-autoexit', '-loglevel', 'error', source], env=env)
    if platform == 'linux':
        run(['ldd', binary], env=env)
    dist = ROOT / 'dist'
    dist.mkdir(exist_ok=True)
    archive = shutil.make_archive(str(dist / name), 'zip' if platform == 'windows' else 'gztar', root_dir=TARGET, base_dir=name)
    path = Path(archive)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    path.with_name(path.name + '.sha256').write_text(f'{digest}  {path.name}\n')
    print('Created and smoke-tested:', path, flush=True)


if __name__ == '__main__':
    operation, platform = sys.argv[1:]
    {'prepare': prepare, 'package': package}[operation](platform)
