#!/usr/bin/env python3
"""Collect third-party source material without exporting easycat source files."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest() if hasattr(hashlib, 'file_digest') else hashlib.sha256(stream.read()).hexdigest()


def collect_mac(record, output):
    destination = output / (record['component'] + ('.tar.gz' if record['git_revision'] else '-' + record['url'].rsplit('/', 1)[1]))
    result = dict(record, filename=destination.name)
    try:
        if record['git_revision']:
            if not destination.exists():
                with tempfile.TemporaryDirectory(prefix='easycat-third-party-') as temporary:
                    subprocess.run(['git', 'init', '-q', temporary], check=True)
                    subprocess.run(['git', '-C', temporary, 'fetch', '-q', '--depth=1', record['url'], record['git_revision']], check=True, timeout=180)
                    actual = subprocess.check_output(['git', '-C', temporary, 'rev-parse', 'FETCH_HEAD'], text=True).strip()
                    if actual != record['git_revision']:
                        raise RuntimeError('Git revision mismatch')
                    subprocess.run(['git', '-C', temporary, 'archive', '--format=tar.gz',
                                    '--prefix=' + record['component'] + '/', '--output=' + str(destination), actual], check=True)
        elif not destination.exists():
            request = urllib.request.Request(record['url'], headers={'User-Agent': 'easycat-source-collector/1'})
            with urllib.request.urlopen(request, timeout=90) as response, destination.with_suffix(destination.suffix + '.partial').open('wb') as stream:
                shutil.copyfileobj(response, stream)
            destination.with_suffix(destination.suffix + '.partial').rename(destination)
        digest = sha256(destination)
        if record['sha256'] and digest != record['sha256']:
            raise RuntimeError('Source checksum differs from the packaged Homebrew recipe')
        result.update(status='verified', sha256_collected=digest)
    except Exception as error:
        result.update(status='blocked', error=str(error))
    return result


def collect_portable(record, output):
    result = dict(record)
    revision = record['revision']
    if not revision or len(revision) != 40 or any(c not in '0123456789abcdef' for c in revision):
        return dict(result, status='blocked', error='Recipe does not pin a full Git commit; resolve original build revision first')
    name = record['recipe'].replace('/', '_').removesuffix('.sh')
    destination = output / (name + '-' + revision + '.tar.gz')
    repository = record['repository'].removesuffix('.git')
    try:
        if not destination.exists():
            if repository.startswith('https://github.com/'):
                url = repository.replace('https://github.com/', 'https://codeload.github.com/') + '/tar.gz/' + revision
                request = urllib.request.Request(url, headers={'User-Agent': 'easycat-source-collector/1'})
                partial = destination.with_suffix('.partial')
                with urllib.request.urlopen(request, timeout=120) as response, partial.open('wb') as stream:
                    shutil.copyfileobj(response, stream)
                partial.rename(destination)
            else:
                # Export the exact object; never execute upstream download/build scripts.
                with tempfile.TemporaryDirectory(prefix='easycat-third-party-') as temporary:
                    subprocess.run(['git', 'init', '-q', temporary], check=True)
                    subprocess.run(['git', '-C', temporary, 'fetch', '-q', '--depth=1', record['repository'], revision], check=True, timeout=120, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    actual = subprocess.check_output(['git', '-C', temporary, 'rev-parse', 'FETCH_HEAD'], text=True).strip()
                    if actual != revision:
                        raise RuntimeError('Git revision mismatch')
                    subprocess.run(['git', '-C', temporary, 'archive', '--format=tar.gz', '--prefix=' + name + '/', '--output=' + str(destination), actual], check=True)
        with tarfile.open(destination) as archive:
            modules = [member for member in archive if member.name.endswith('/.gitmodules') and member.size]
        result.update(filename=destination.name, sha256_collected=sha256(destination), status='collected')
        if modules:
            result.update(status='blocked', error='Source contains submodules; corresponding submodule trees must be collected and checked')
    except Exception as error:
        result.update(status='blocked', error=str(error))
    return result


def collect_crates(metadata, output):
    records = []
    # Cargo metadata identifies registry packages. Workspace packages are never exported.
    for package in metadata['packages']:
        if not package.get('source'):
            continue
        if package['source'] != 'registry+https://github.com/rust-lang/crates.io-index':
            raise RuntimeError('Only public crates.io dependencies may enter the public source bundle')
        if not (package.get('source') or '').startswith('registry+'):
            continue
        manifest = Path(package['manifest_path'])
        registry = manifest.parent.parent
        cache = registry.parent.parent / 'cache' / registry.name
        archive = cache / (package['name'] + '-' + package['version'] + '.crate')
        if not archive.is_file():
            raise RuntimeError(f'Missing cached third-party crate: {archive.name}; run cargo metadata online first')
        records.append({'name': package['name'], 'version': package['version'],
                        'license_expression': package['license'], 'filename': archive.name,
                        'sha256': sha256(archive)})
    destination = output / 'rust-third-party-sources.tar.gz'
    if destination.exists():
        raise RuntimeError(f'Refusing to overwrite {destination}')
    with tarfile.open(destination, 'w:gz') as bundle:
        for record in records:
            package = next(p for p in metadata['packages'] if p['name'] == record['name'] and p['version'] == record['version'])
            registry = Path(package['manifest_path']).parent.parent
            archive = registry.parent.parent / 'cache' / registry.name / record['filename']
            bundle.add(archive, arcname='crates/' + archive.name)
    (output / 'rust-third-party-index.json').write_text(json.dumps(records, indent=2) + '\n')
    return len(records)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--portable', action='store_true', help='Collect pinned portable-build source candidates; incomplete trees remain blocked')
    parser.add_argument('--macos', action='store_true', help='Download pinned macOS dependency sources')
    parser.add_argument('--cargo-metadata', type=Path, help='Include cached registry crate archives, never workspace sources')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist/third-party-sources')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if args.cargo_metadata:
        print(f'Collected {collect_crates(json.loads(args.cargo_metadata.read_text()), output)} third-party crate archives')
    if args.portable:
        records = json.loads((ROOT / 'distribution/compliance/portable-source-candidates.json').read_text())['candidates']
        with ThreadPoolExecutor(max_workers=5) as workers:
            results = list(workers.map(lambda record: collect_portable(record, output), records))
        (output / 'portable-source-results.json').write_text(json.dumps(results, indent=2) + '\n')
        blocked = [result for result in results if result['status'] == 'blocked']
        print(f'Portable candidates: {len(results) - len(blocked)} collected; {len(blocked)} blocked')
        if blocked:
            raise SystemExit('Portable corresponding-source review is incomplete; see portable-source-results.json')
    if args.macos:
        records = json.loads((ROOT / 'distribution/compliance/macos-sources.json').read_text())
        with ThreadPoolExecutor(max_workers=5) as workers:
            results = list(workers.map(lambda record: collect_mac(record, output), records))
        (output / 'macos-source-results.json').write_text(json.dumps(results, indent=2) + '\n')
        for result in results:
            print(result['component'] + ': ' + result['status'], flush=True)
        if any(result['status'] != 'verified' for result in results):
            raise SystemExit('Some source downloads are blocked; do not publish as complete corresponding source')


if __name__ == '__main__':
    main()
