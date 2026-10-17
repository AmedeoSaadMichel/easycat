#!/usr/bin/env python3
"""Check actual native app binaries for direct FFmpeg library dependencies."""
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tarfile
import tempfile
import zipfile

from package_portable import windows_imports

ROOT = Path(__file__).resolve().parents[1]
FFMPEG_LIBRARIES = ('avcodec', 'avdevice', 'avfilter', 'avformat', 'avutil', 'swresample', 'swscale')


def elf_imports(data):
    if data[:6] != b'\x7fELF\x02\x01':
        raise ValueError('Expected a 64-bit little-endian ELF executable')
    start = struct.unpack_from('<Q', data, 32)[0]
    size, count = struct.unpack_from('<HH', data, 54)
    headers = [struct.unpack_from('<IIQQQQQQ', data, start + i * size) for i in range(count)]
    dynamic = next(header for header in headers if header[0] == 2)
    entries = [struct.unpack_from('<qQ', data, position) for position in range(dynamic[2], dynamic[2] + dynamic[5], 16)]
    table = next(value for tag, value in entries if tag == 5)
    segment = next(header for header in headers if header[0] == 1 and header[3] <= table < header[3] + header[5])
    table = segment[2] + table - segment[3]
    names = []
    for tag, value in entries:
        if tag == 1:
            start = table + value
            names.append(data[start:data.index(0, start)].decode())
    return names


def reject_ffmpeg_imports(names):
    for name in names:
        base = Path(name).name.lower().removeprefix('lib')
        if base.startswith(FFMPEG_LIBRARIES):
            raise RuntimeError('App directly links a GPL FFmpeg library: ' + name)


def main():
    reports = {}
    mac = ROOT / 'target/macos-package/easycat.app/Contents/MacOS/easycat-bin'
    dependencies = subprocess.check_output(['otool', '-L', str(mac)], text=True).splitlines()[1:]
    reports['macos'] = {'sha256': hashlib.sha256(mac.read_bytes()).hexdigest(), 'libraries': [line.strip().split(' (')[0] for line in dependencies]}
    with tempfile.TemporaryDirectory(prefix='easycat-link-audit-') as temporary:
        windows = Path(temporary) / 'easycat.exe'
        with zipfile.ZipFile(ROOT / 'dist/easycat-0.1.0-windows-x64.zip') as bundle:
            windows.write_bytes(bundle.read(next(name for name in bundle.namelist() if name.endswith('/easycat.exe'))))
        reports['windows'] = {'sha256': hashlib.sha256(windows.read_bytes()).hexdigest(), 'libraries': windows_imports(windows)}
    with tarfile.open(ROOT / 'dist/easycat-0.1.0-linux-x86_64.tar.gz') as bundle:
        data = bundle.extractfile(next(member for member in bundle if member.name.endswith('/easycat-bin'))).read()
        reports['linux'] = {'sha256': hashlib.sha256(data).hexdigest(), 'libraries': elf_imports(data)}
    for report in reports.values():
        reject_ffmpeg_imports(report['libraries'])
        report['direct_ffmpeg_library_linkage'] = False
    destination = ROOT / 'distribution/compliance/app-linkage-audit.json'
    destination.write_text(json.dumps(reports, indent=2) + '\n')
    print('Actual macOS, Windows and Linux application binaries do not directly link FFmpeg libraries')


if __name__ == '__main__':
    main()
