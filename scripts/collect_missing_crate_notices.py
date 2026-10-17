#!/usr/bin/env python3
"""Recover omitted crate license files from the exact public upstream VCS revision."""
from concurrent.futures import ThreadPoolExecutor
import io
import json
from pathlib import Path
import re
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'distribution/crate-upstream-notices'


def collect(item):
    repository, revision, crates = item
    result = {'repository': repository, 'revision': revision, 'crates': crates, 'files': []}
    try:
        url = 'https://codeload.github.com/' + repository + '/tar.gz/' + revision
        with urllib.request.urlopen(url, timeout=120) as response:
            data = response.read()
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            for member in archive:
                relative = Path(*Path(member.name).parts[1:])
                if not member.isfile() or relative.is_absolute() or '..' in relative.parts:
                    continue
                if not relative.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'COPYRIGHT', 'NOTICE', 'AUTHORS')):
                    continue
                target = OUT / (repository.replace('/', '_') + '-' + revision) / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.extractfile(member).read())
                result['files'].append(str(target.relative_to(OUT)))
        result['status'] = 'collected' if result['files'] else 'blocked'
    except Exception as error:
        result.update(status='blocked', error=str(error))
    return result


def main():
    metadata = json.loads((ROOT / 'target/license-audit/cargo-metadata.json').read_text())
    index = json.loads((ROOT / 'target/distribution-notices/rust-index.json').read_text())
    missing = {(p['name'], p['version']) for p in index if not p['notice_files']}
    groups = {}
    unresolved = []
    for package in metadata['packages']:
        if (package['name'], package['version']) not in missing:
            continue
        if package.get('source') != 'registry+https://github.com/rust-lang/crates.io-index':
            raise RuntimeError('Only public registry packages may be inspected')
        vcs = Path(package['manifest_path']).parent / '.cargo_vcs_info.json'
        repo = re.match(r'https?://github.com/([^/]+/[^/#]+)', package.get('repository') or '')
        name = package['name'] + '-' + package['version']
        if not repo:
            unresolved.append(name)
            continue
        if vcs.exists():
            revision = json.loads(vcs.read_text())['git']['sha1']
        elif package['name'] in ('block', 'malloc_buf'):
            revision = package['version']
        elif package['name'].startswith('winapi-'):
            revision = '0.3.9'
        else:
            unresolved.append(name)
            continue
        groups.setdefault((repo[1].removesuffix('.git'), revision), []).append(name)
    OUT.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=5) as workers:
        results = list(workers.map(collect, [(repo, rev, crates) for (repo, rev), crates in groups.items()]))
    (OUT / 'index.json').write_text(json.dumps({'upstreams': results, 'unresolved': unresolved}, indent=2) + '\n')
    print(f'Collected {sum(p["status"] == "collected" for p in results)} upstream notice sets; unresolved crates: {unresolved}')
    return 1 if unresolved or any(p['status'] == 'blocked' for p in results) else 0


if __name__ == '__main__':
    raise SystemExit(main())
