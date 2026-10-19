#!/usr/bin/env python3
"""Report official-cask release blockers; never publish or change repository visibility."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def command(*args):
    return subprocess.run(args, text=True, capture_output=True, check=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', default='AmedeoSaadMichel/easycat')
    parser.add_argument('--tag', default='v0.1.0-preview.1')
    parser.add_argument('--dmg', type=Path)
    args = parser.parse_args()
    checks = []

    def record(name, passed, detail):
        checks.append({'check': name, 'passed': passed, 'detail': detail})

    repo = command('gh', 'api', f'repos/{args.repo}')
    if repo.returncode:
        record('upstream metadata', False, 'Could not query GitHub; authenticate gh and retry')
    else:
        data = json.loads(repo.stdout)
        record('public upstream', not data['private'], data['html_url'])
        age = (datetime.now(timezone.utc) - datetime.fromisoformat(data['created_at'].replace('Z', '+00:00'))).days
        record('repository age', age >= 30, f'{age} days; normal minimum is 30')
        stars, forks, watchers = (data[key] for key in ('stargazers_count', 'forks_count', 'subscribers_count'))
        record('owner-submission public interest', stars >= 225 or forks >= 90 or watchers >= 90,
               f'{stars} stars, {forks} forks, {watchers} watchers; normal thresholds: 225/90/90 (any one)')
    release = command('gh', 'api', f'repos/{args.repo}/releases/tags/{args.tag}')
    if release.returncode:
        record('release', False, 'Release metadata unavailable')
    else:
        data = json.loads(release.stdout)
        record('planned general-user release', not data['draft'] and not data['prerelease'],
               'Project plans a stable default channel; a preview is not universally prohibited by Homebrew')
        if args.dmg:
            asset = next((item for item in data['assets'] if item['name'] == args.dmg.name), None)
            actual = 'sha256:' + hashlib.sha256(args.dmg.read_bytes()).hexdigest()
            record('published DMG checksum', bool(asset and asset.get('digest') == actual), actual)
    identities = command('security', 'find-identity', '-v', '-p', 'codesigning')
    record('Developer ID Application certificate', identities.returncode == 0 and
           '"Developer ID Application:' in identities.stdout, 'Must be available on the signing Mac')
    if args.dmg:
        for name, invocation in [
            ('DMG staple', ('xcrun', 'stapler', 'validate', str(args.dmg))),
            ('DMG Gatekeeper', ('spctl', '--assess', '--type', 'open', '--context',
                               'context:primary-signature', str(args.dmg))),
        ]:
            result = command(*invocation)
            record(name, result.returncode == 0, (result.stdout + result.stderr).strip())
    record('project license file', (ROOT / 'LICENSE').is_file(),
           'Cargo declares MIT; the owner must confirm the corresponding license/copyright text')
    record('human release review', False,
           'Confirm dependency source/license obligations, clean supported-Mac tests, name search and official cask audit; this script cannot certify these')
    print(json.dumps({'ready_for_submission': all(item['passed'] for item in checks),
                      'checks': checks}, indent=2))
    return 0 if all(item['passed'] for item in checks) else 1


if __name__ == '__main__':
    raise SystemExit(main())
