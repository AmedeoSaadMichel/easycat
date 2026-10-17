#!/usr/bin/env python3
"""Verify four upstream SPDX declarations and generate complete attribution notices."""
import hashlib
import json
from pathlib import Path
import re
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
CASES = {
    'block': ('0.1.6', 'MIT', 'SSheldon/rust-block', '47178790cfc9d4a8b092051d8b413b78bd31254a', 'Cargo.toml'),
    'dispatch': ('0.2.0', 'MIT', 'SSheldon/rust-dispatch', '82d6c7a5b75dc0c71c3f46f87bb6c16a476f7748', 'Cargo.toml'),
    'malloc_buf': ('0.0.6', 'MIT', 'SSheldon/malloc_buf', 'a7811e5f4c6f9685dd57ad9bfe34e9bf6b0ba4f9', 'Cargo.toml'),
    'hexf-parse': ('0.2.1', 'CC0-1.0', 'lifthrasiir/hexf', '4225763d744183d720f575ae96d04161b4d08ea0', 'parse/Cargo.toml'),
}


def field(text, name):
    package = re.split(r'^\[package\]\s*$', text, flags=re.M)[1]
    package = re.split(r'^\[', package, flags=re.M)[0]
    match = re.search(r'^' + re.escape(name) + r'\s*=\s*(.+)$', package, re.M)
    if not match:
        raise ValueError('Missing upstream package field: ' + name)
    return json.loads(match[1].strip())


def verify_declaration(text, name, version, license_name):
    for key, expected in [('name', name), ('version', version), ('license', license_name)]:
        if field(text, key) != expected:
            raise ValueError('Upstream declaration mismatch: ' + key)
    authors = field(text, 'authors')
    if not authors or not all(isinstance(author, str) and author for author in authors):
        raise ValueError('Missing upstream author attribution')
    return authors


def main():
    metadata = json.loads((ROOT / 'target/license-audit/cargo-metadata.json').read_text())
    lock = (ROOT / 'Cargo.lock').read_text()
    output = ROOT / 'distribution/resolved-crate-licenses'
    output.mkdir(parents=True, exist_ok=True)
    results = []
    for name, (version, license_name, repository, revision, manifest_name) in CASES.items():
        package = next(p for p in metadata['packages'] if p['name'] == name and p['version'] == version)
        if package['source'] != 'registry+https://github.com/rust-lang/crates.io-index':
            raise ValueError('Unexpected registry')
        source = Path(package['manifest_path']).parent
        registry = source.parent
        archive = registry.parent.parent / 'cache' / registry.name / f'{name}-{version}.crate'
        checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
        entries = [entry for entry in lock.split('[[package]]') if re.search(r'^name = "' + re.escape(name) + '"$', entry, re.M) and f'version = "{version}"' in entry]
        if len(entries) != 1 or f'checksum = "{checksum}"' not in entries[0]:
            raise ValueError('Published crate checksum does not match Cargo.lock')
        with tarfile.open(archive) as bundle:
            manifest = bundle.extractfile(f'{name}-{version}/Cargo.toml').read().decode()
        authors = verify_declaration(manifest, name, version, license_name)
        url = f'https://raw.githubusercontent.com/{repository}/{revision}/{manifest_name}'
        with urllib.request.urlopen(url, timeout=30) as response:
            upstream = response.read().decode()
        if verify_declaration(upstream, name, version, license_name) != authors:
            raise ValueError('Upstream and published author attribution differ')
        folder = output / f'{name}-{version}'
        folder.mkdir(exist_ok=True)
        (folder / 'published-Cargo.toml').write_text(manifest)
        (folder / 'upstream-Cargo.toml').write_text(upstream)
        standard = (ROOT / f'distribution/standard-licenses/{license_name}.txt').read_text()
        # MIT's canonical SPDX template contains a placeholder, not an upstream notice.
        # Preserve actual attribution separately and include the complete permission text.
        text = standard[standard.index('Permission is hereby'): ] if license_name == 'MIT' else standard
        (folder / 'NOTICE.txt').write_text(
            f'{name} {version}\nUpstream license: {license_name}\n'
            f'Author attribution from the published manifest: {", ".join(authors)}\n'
            f'Upstream declaration: {url}\nPublished crate SHA-256: {checksum}\n\n'
            'This notice supplies the text of the license explicitly declared by upstream.\n'
            'No copyright year or additional permission is invented. Original source and\n'
            'any original notices are preserved in the accompanying crate archive.\n\n' + text)
        results.append({'crate': f'{name}-{version}', 'license': license_name, 'authors': authors,
                        'upstream_commit': revision, 'upstream_manifest_url': url,
                        'crate_sha256': checksum, 'published_manifest_sha256': hashlib.sha256(manifest.encode()).hexdigest(),
                        'upstream_manifest_sha256': hashlib.sha256(upstream.encode()).hexdigest(),
                        'resolution': 'verified-upstream-SPDX-declaration-with-complete-notice',
                        'status': 'resolved', 'notice': f'{name}-{version}/NOTICE.txt'})
    (output / 'resolutions.json').write_text(json.dumps(results, indent=2) + '\n')
    print('Four license cases resolved from matching published and pinned upstream declarations')


if __name__ == '__main__':
    main()
