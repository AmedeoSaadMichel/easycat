#!/usr/bin/env python3
"""Rebuild the preserved macOS FFmpeg source with its recorded codec configuration."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / 'target/license-audit/native-rebuild/macos'


def main():
    records = json.loads((ROOT / 'distribution/compliance/macos-source-results.json').read_text())
    record = next(r for r in records if r['component'].startswith('ffmpeg-'))
    archive = ROOT / 'dist/third-party-sources' / record['filename']
    if hashlib.sha256(archive.read_bytes()).hexdigest() != record['sha256_collected']:
        raise RuntimeError('Preserved FFmpeg source checksum mismatch')
    WORK.mkdir(parents=True, exist_ok=True)
    source = WORK / 'ffmpeg-9.0.2'
    if not source.exists():
        with tarfile.open(archive) as bundle:
            for member in bundle:
                if not (WORK / member.name).resolve().is_relative_to(WORK.resolve()):
                    raise RuntimeError('Unsafe source member')
            bundle.extractall(WORK)
    prefix = WORK / 'install'
    flags = [line.strip() for line in (ROOT / 'distribution/compliance/macos-ffmpeg-buildconf.txt').read_text().splitlines() if line.strip().startswith('--')]
    flags = [f'--prefix={prefix}' if flag.startswith('--prefix=') else flag for flag in flags]
    flags += ['--extra-cflags=-I/opt/homebrew/include', '--extra-ldflags=-L/opt/homebrew/lib']
    pc = WORK / 'pkgconfig'
    pc.mkdir(exist_ok=True)
    for dependency in records:
        formula = ROOT / 'target/macos-package/easycat.app/Contents/Resources/Licenses' / dependency['component']
        receipt = json.loads((formula / 'INSTALL_RECEIPT.json').read_text())
        package, version = dependency['component'].rsplit('-', 1)
        for directory in [Path('/opt/homebrew/Cellar') / package / version / 'lib/pkgconfig', Path('/opt/homebrew/Cellar') / package / version / 'share/pkgconfig']:
            if directory.is_dir():
                for entry in directory.glob('*.pc'):
                    shutil.copy2(entry, pc / entry.name)
    env = dict(os.environ, PKG_CONFIG_LIBDIR=str(pc), PATH='/opt/homebrew/bin:' + os.environ.get('PATH', ''), PKG_CONFIG_PATH=str(pc))
    log = WORK / 'build.log'
    with log.open('w') as output:
        for command in [['./configure', *flags], ['make', '-j8'], ['make', 'install']]:
            output.write('COMMAND ' + json.dumps(command) + '\n');output.flush()
            subprocess.run(command, cwd=source, env=env, stdout=output, stderr=subprocess.STDOUT, check=True)
    tools = prefix / 'bin'
    license_text = subprocess.check_output([str(tools / 'ffmpeg'), '-hide_banner', '-L'], text=True, stderr=subprocess.STDOUT)
    if 'version 3' not in license_text:
        raise RuntimeError('Rebuilt license does not match GPLv3')
    (WORK / 'result.json').write_text(json.dumps({'status': 'built', 'source_sha256': record['sha256_collected'], 'configuration': flags,
        'compiler': subprocess.check_output(['clang', '--version'], text=True).splitlines()[0],
        'dependency_scope': 'Rebuilt FFmpeg; recorded Homebrew dependency binaries are used for linking',
        'ffmpeg_sha256': hashlib.sha256((tools / 'ffmpeg').read_bytes()).hexdigest()}, indent=2) + '\n')
    print('macOS FFmpeg rebuilt with recorded codec flags:', prefix)


if __name__ == '__main__':
    main()
