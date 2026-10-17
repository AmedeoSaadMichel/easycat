#!/usr/bin/env python3
"""Collect registry sources used by the pinned FFmpeg Rust dependencies."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'dist/third-party-sources'


def main():
    records = json.loads((OUT / 'portable-source-results.json').read_text())
    results = []
    for recipe in ('50-rav1e.sh', '50-librsvg/99-librsvg.sh'):
        record = next(row for row in records if row['recipe'] == recipe)
        if record['status'] != 'collected':
            raise RuntimeError(f'Source tree incomplete: {recipe}')
        name = recipe.replace('/', '_').removesuffix('.sh')
        destination = OUT / (name + '-cargo-vendor.tar.gz')
        if destination.exists():
            raise RuntimeError(f'Refusing to overwrite {destination}')
        with tempfile.TemporaryDirectory(prefix='easycat-ffmpeg-rust-source-') as temporary:
            work = Path(temporary).resolve()
            with tarfile.open(OUT / record['filename']) as archive:
                for member in archive:
                    path = work / member.name
                    if not path.resolve().is_relative_to(work):
                        raise RuntimeError('Unsafe source archive path')
                    if member.issym() and not (path.parent / member.linkname).resolve().is_relative_to(work):
                        raise RuntimeError('Unsafe source archive symlink')
                    if member.islnk() and not (work / member.linkname).resolve().is_relative_to(work):
                        raise RuntimeError('Unsafe source archive hardlink')
                archive.extractall(work)
            source = next(path for path in work.iterdir() if path.is_dir())
            vendor = work / 'vendor'
            env = dict(os.environ, RUSTUP_TOOLCHAIN='stable')
            config = subprocess.check_output(['cargo', 'vendor', '--locked', '--versioned-dirs',
                                               '--manifest-path', str(source / 'Cargo.toml'), str(vendor)],
                                              cwd=source, env=env, text=True, timeout=600)
            # Replace temporary paths with an archive-relative vendor directory.
            config = config.replace(str(vendor), 'vendor')
            config_file = work / 'cargo-config.toml'
            config_file.write_text(config)
            with tarfile.open(destination, 'w:gz') as archive:
                archive.add(vendor, arcname='vendor')
                archive.add(config_file, arcname='cargo-config.toml')
            results.append({'recipe': recipe, 'filename': destination.name,
                            'sha256': hashlib.sha256(destination.read_bytes()).hexdigest(),
                            'status': 'collected'})
    (OUT / 'ffmpeg-rust-vendor-results.json').write_text(json.dumps(results, indent=2) + '\n')
    print('Collected locked Cargo dependency sources for rav1e and librsvg')


if __name__ == '__main__':
    main()
