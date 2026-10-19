#!/usr/bin/env python3
"""Prepare public third-party notices, excluding workspace/private-registry content."""
import json
import hashlib
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PUBLIC_REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'


def is_notice_path(path):
    path = Path(path)
    if path.suffix.lower() in ('.rs', '.c', '.cpp', '.h', '.py', '.js'):
        return False
    if any(part.upper() in ('LICENSES', 'LICENCES', 'LICENSE', 'LICENCE') for part in path.parts[:-1]):
        return True
    name = path.name.upper()
    if any(term in name for term in ('LICENSE', 'LICENCE', 'COPYRIGHT')):
        return True
    if name.startswith(('COPYING', 'NOTICE', 'AUTHORS', 'OFL', 'UFL')):
        return True
    return path.suffix.lower() == '.txt' and any(part.lower() == 'fonts' for part in path.parts)


def prepare_notices(destination, metadata=None):
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    if metadata is None:
        compiler = subprocess.check_output(['rustc', '-vV'], text=True)
        host = next(line.split(': ', 1)[1] for line in compiler.splitlines() if line.startswith('host: '))
        metadata = json.loads(subprocess.check_output(
            ['cargo', 'metadata', '--locked', '--offline', '--format-version', '1', '--filter-platform', host],
            cwd=ROOT, text=True))
    resolution_root = ROOT / 'distribution/resolved-crate-licenses'
    resolutions = {item['crate']: item for item in json.loads((resolution_root / 'resolutions.json').read_text())}
    records = []
    for package in metadata['packages']:
        if not package.get('source'):
            continue
        if package['source'] != PUBLIC_REGISTRY:
            raise RuntimeError('Refusing to export notices from a private or unreviewed registry')
        source = Path(package['manifest_path']).parent.resolve()
        name = package['name'] + '-' + package['version']
        resolution = resolutions.get(name)
        if resolution and (package['license'] != resolution['license'] or
                           hashlib.sha256(Path(package['manifest_path']).read_bytes()).hexdigest() != resolution['published_manifest_sha256']):
            raise RuntimeError('Resolved license evidence does not match published package: ' + name)
        license_files = []
        for path in sorted(source.rglob('*')):
            if not path.is_file() or path.is_symlink():
                continue
            if not is_notice_path(path.relative_to(source)):
                continue
            relative = path.relative_to(source)
            target = destination / 'rust-licenses' / name / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
            license_files.append(str(relative).replace('\\', '/'))
        records.append({'name': package['name'], 'version': package['version'],
                        'license_expression': package['license'], 'notice_files': license_files,
                        'source_archive': name + '.crate',
                        'verified_license_notice': ('resolved-crate-licenses/' + resolution['notice']) if resolution else None})
    shutil.copytree(resolution_root, destination / 'resolved-crate-licenses')
    shutil.copytree(ROOT / 'distribution/standard-licenses', destination / 'standard-licenses')
    shutil.copy2(ROOT / 'distribution/compliance/metadata-only-crate-licenses.json', destination)
    tool_notices = ROOT / 'distribution/tool-dependency-notices'
    if tool_notices.is_dir():
        shutil.copytree(tool_notices, destination / 'tool-dependency-notices')
    extra = ROOT / 'distribution/crate-upstream-notices'
    if extra.is_dir():
        shutil.copytree(extra, destination / 'rust-upstream-notices')
    (destination / 'rust-index.json').write_text(json.dumps(records, indent=2) + '\n')
    shutil.copy2(ROOT / 'distribution/THIRD-PARTY-NOTICES.md', destination)
    shutil.copy2(ROOT / 'distribution/compliance/GPL-3.0.txt', destination)
    for font in (ROOT / 'easycat-app/assets').glob('*OFL.txt'):
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
