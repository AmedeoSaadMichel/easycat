#!/usr/bin/env python3
"""Prepare public third-party notices, excluding workspace/private-registry content."""
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PUBLIC_REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'


def prepare_notices(destination, metadata=None):
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    if metadata is None:
        compiler = subprocess.check_output(['rustc', '-vV'], text=True)
        host = next(line.split(': ', 1)[1] for line in compiler.splitlines() if line.startswith('host: '))
        metadata = json.loads(subprocess.check_output(
            ['cargo', 'metadata', '--locked', '--offline', '--format-version', '1', '--filter-platform', host],
            cwd=ROOT, text=True))
    records = []
    for package in metadata['packages']:
        if not package.get('source'):
            continue
        if package['source'] != PUBLIC_REGISTRY:
            raise RuntimeError('Refusing to export notices from a private or unreviewed registry')
        source = Path(package['manifest_path']).parent.resolve()
        name = package['name'] + '-' + package['version']
        license_files = []
        for path in sorted(source.rglob('*')):
            if not path.is_file() or path.is_symlink():
                continue
            if not path.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'COPYRIGHT', 'NOTICE', 'AUTHORS')):
                continue
            relative = path.relative_to(source)
            target = destination / 'rust-licenses' / name / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
            license_files.append(str(relative).replace('\\', '/'))
        records.append({'name': package['name'], 'version': package['version'],
                        'license_expression': package['license'], 'notice_files': license_files,
                        'source_archive': name + '.crate'})
    shutil.copytree(ROOT / 'distribution/standard-licenses', destination / 'standard-licenses')
    shutil.copy2(ROOT / 'distribution/compliance/metadata-only-crate-licenses.json', destination)
    extra = ROOT / 'distribution/crate-upstream-notices'
    if extra.is_dir():
        shutil.copytree(extra, destination / 'rust-upstream-notices')
    (destination / 'rust-index.json').write_text(json.dumps(records, indent=2) + '\n')
    shutil.copy2(ROOT / 'distribution/THIRD-PARTY-NOTICES.md', destination)
    shutil.copy2(ROOT / 'distribution/compliance/GPL-3.0.txt', destination)
    for font in (ROOT / 'easycut-app/assets').glob('*OFL.txt'):
        shutil.copy2(font, destination)
    # Registry archives may use Unix epoch timestamps, which ZIP cannot represent.
    for file in destination.rglob('*'):
        if file.is_file():
            os.utime(file, (315532800, 315532800))
    return records


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--cargo-metadata', type=Path)
    args = parser.parse_args()
    metadata = json.loads(args.cargo_metadata.read_text()) if args.cargo_metadata else None
    records = prepare_notices(args.output, metadata)
    print(f'Prepared notices for {len(records)} public third-party crates; workspace sources excluded')
