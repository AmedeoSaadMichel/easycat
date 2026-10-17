#!/usr/bin/env python3
"""Native source-build sanity check of the pinned portable FFmpeg core."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
SHA = 'e6cfdd7b8c708239dcc39a71fc454c44b8c95de7b74d014c2f4ba4e06a6a0c52'
COMMIT = '669f5ab083c7732c10f1ce9e4f1915960dd88457'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work', type=Path, default=ROOT / 'target/source-build-check')
    args = parser.parse_args()
    work = args.work.resolve()
    work.mkdir(parents=True, exist_ok=True)
    archive = work / 'ffmpeg-source.tar.gz'
    if not archive.exists():
        urllib.request.urlretrieve('https://api.github.com/repos/FFmpeg/FFmpeg/tarball/' + COMMIT, archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA:
        raise RuntimeError('Pinned source checksum mismatch')
    with tarfile.open(archive) as bundle:
        for member in bundle:
            if not (work / member.name).resolve().is_relative_to(work):
                raise RuntimeError('Unsafe source path')
        bundle.extractall(work)
    source = next(path for path in work.iterdir() if path.is_dir() and path.name.startswith('FFmpeg-'))
    prefix = work / 'install'
    # This deliberately tests the supplied core source on the native compiler.
    # Full external-codec configurations are tested using the distributed binaries.
    install_prefix = str(prefix)
    if platform.system() == 'Windows':
        install_prefix = subprocess.check_output(['cygpath', '-u', str(prefix)], text=True).strip()
    flags = ['--enable-gpl', '--enable-version3', '--disable-autodetect', '--disable-doc',
             '--disable-ffplay', '--disable-x86asm', '--enable-shared', '--disable-static',
             '--prefix=' + install_prefix]
    if platform.system() == 'Windows':
        flags += ['--target-os=mingw32', '--arch=x86_64', '--cc=gcc']
    else:
        flags += ['--cc=gcc']
    with (work / 'build.log').open('w') as log:
        for command in [['bash', './configure', *flags], ['make', '-j4'], ['make', 'install']]:
            subprocess.run(command, cwd=source, stdout=log, stderr=subprocess.STDOUT, check=True)
    env = os.environ.copy()
    env['PATH'] = str(prefix / 'bin') + os.pathsep + env.get('PATH', '')
    env['LD_LIBRARY_PATH'] = str(prefix / 'lib')
    ffmpeg = prefix / 'bin' / ('ffmpeg.exe' if platform.system() == 'Windows' else 'ffmpeg')
    fixture = work / 'source-roundtrip.mp4'
    subprocess.run([str(ffmpeg), '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=160x96:rate=10',
                    '-t', '0.3', '-c:v', 'mpeg4', '-y', str(fixture)], env=env, check=True)
    subprocess.run([str(ffmpeg), '-v', 'error', '-i', str(fixture), '-f', 'null', '-'], env=env, check=True)
    (work / 'result.json').write_text(json.dumps({'status': 'passed', 'platform': platform.system(),
        'source_commit': COMMIT, 'source_sha256': SHA, 'configuration': flags,
        'scope': 'Native core compilation and encode/decode roundtrip; not a rebuild of BtbN external libraries',
        'full_upstream_configuration_rebuilt': False}, indent=2) + '\n')
    print('Pinned FFmpeg core builds and roundtrips on', platform.system())


if __name__ == '__main__':
    main()
