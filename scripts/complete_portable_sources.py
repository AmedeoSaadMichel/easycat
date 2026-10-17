#!/usr/bin/env python3
"""Resolve recipe tags/submodules into source archives without running build scripts."""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'dist/third-party-sources'


def git(directory, *args):
    return subprocess.check_output(['git', '-c', 'core.hooksPath=/dev/null',
                                    '-c', 'protocol.file.allow=never', '-C', str(directory),
                                    *args], text=True, stderr=subprocess.PIPE, timeout=180)


def collect(record):
    result = dict(record)
    if not record['revision']:
        return result
    name = record['recipe'].replace('/', '_').removesuffix('.sh')
    try:
        with tempfile.TemporaryDirectory(prefix='easycat-third-party-') as temporary:
            checkout = Path(temporary)
            if record.get('vcs') == 'svn':
                source = checkout / 'source'
                subprocess.run(['svn', 'export', '--non-interactive', '--username', 'anonymous', '--password', '',
                                '-r', record['revision'], record['repository'] + '@' + record['revision'], str(source)],
                               check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, timeout=180)
                destination = OUT / (name + '-svn-r' + record['revision'] + '.tar.gz')
                with tarfile.open(destination, 'w:gz') as archive:
                    archive.add(source, arcname=name)
                result.update(status='collected', filename=destination.name,
                              sha256_collected=hashlib.sha256(destination.read_bytes()).hexdigest())
                result.pop('error', None)
                return result
            git(checkout, 'init', '-q')
            git(checkout, 'remote', 'add', 'origin', record['repository'])
            git(checkout, 'fetch', '-q', '--depth=1', 'origin', record['revision'])
            commit = git(checkout, 'rev-parse', 'FETCH_HEAD').strip()
            git(checkout, 'checkout', '-q', '--detach', commit)
            git(checkout, 'submodule', 'update', '--init', '--recursive', '--depth=1', '--checkout')
            modules = git(checkout, 'submodule', 'status', '--recursive')
            if any(line.startswith(('-', '+', 'U')) for line in modules.splitlines()):
                raise RuntimeError('Submodule state does not match pinned source tree: ' + modules)
            destination = OUT / (name + '-' + commit + '-complete-tree.tar.gz')
            def omit_git(member):
                return None if '.git' in Path(member.name).parts else member
            with tarfile.open(destination, 'w:gz') as archive:
                archive.add(checkout, arcname=name, filter=omit_git)
            result.update(status='collected', filename=destination.name,
                          resolved_revision=commit, submodules=modules.splitlines(),
                          sha256_collected=hashlib.sha256(destination.read_bytes()).hexdigest())
            result.pop('error', None)
    except Exception as error:
        result.update(status='blocked', error=str(error) + ('\n' + error.stderr if isinstance(error, subprocess.CalledProcessError) and error.stderr else ''))
    return result


def main():
    records = json.loads((OUT / 'portable-source-results.json').read_text())
    pending = [record for record in records if record['status'] == 'blocked']
    with ThreadPoolExecutor(max_workers=4) as workers:
        completed = {record['recipe']: record for record in workers.map(collect, pending)}
    results = [completed.get(record['recipe'], record) for record in records]
    (OUT / 'portable-source-results.json').write_text(json.dumps(results, indent=2) + '\n')
    blocked = [record for record in results if record['status'] == 'blocked']
    print(f'{len(results) - len(blocked)} source candidates collected; {len(blocked)} blocked')
    for record in blocked:
        print(record['recipe'] + ': ' + record.get('error', 'Unresolved'))
    return 1 if blocked else 0


if __name__ == '__main__':
    raise SystemExit(main())
