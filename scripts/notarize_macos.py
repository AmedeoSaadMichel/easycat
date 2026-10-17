#!/usr/bin/env python3
"""Copy, Developer ID sign and notarize an existing self-contained easycat app."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

MACHO = {b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xcf',
         b'\xfe\xed\xfa\xce', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca',
         b'\xca\xfe\xba\xbf', b'\xbf\xba\xfe\xca'}


def run(*args):
    return subprocess.check_output([str(arg) for arg in args], text=True)


def macho_files(app):
    for path in sorted(app.rglob('*')):
        if path.is_file() and not path.is_symlink():
            with path.open('rb') as stream:
                if stream.read(4) in MACHO:
                    yield path


def notarize(path, profile, log):
    response = json.loads(run('xcrun', 'notarytool', 'submit', path,
                              '--keychain-profile', profile, '--wait', '--output-format', 'json'))
    log.write_text(run('xcrun', 'notarytool', 'log', response['id'],
                       '--keychain-profile', profile))
    if response.get('status') != 'Accepted':
        raise RuntimeError(f'Notarization rejected; see {log}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True, help='New DMG path; never overwritten')
    parser.add_argument('--identity', required=True, help='Developer ID Application certificate name')
    parser.add_argument('--keychain-profile', required=True, help='Existing notarytool credential profile')
    parser.add_argument('--plan', action='store_true', help='Inspect input without signing, uploading or writing')
    args = parser.parse_args()
    app = args.app.resolve(strict=True)
    output = args.output.absolute()
    reserved = [output, output.with_suffix('.dmg.sha256'),
                output.with_name(output.name + '.app-notary-log.json'),
                output.with_name(output.name + '.notary-log.json')]
    if output.suffix != '.dmg' or any(path.exists() for path in reserved):
        parser.error('Choose a new .dmg output path with no existing checksum')
    if app.name != 'easycat.app' or not (app / 'Contents/Info.plist').is_file():
        parser.error('Input must be a packaged easycat.app')
    if not args.identity.startswith('Developer ID Application:'):
        parser.error('An Apple Development or ad-hoc identity cannot notarize this release')
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    binaries = list(macho_files(app))
    if not binaries:
        parser.error('No Mach-O executables found')
    if args.plan:
        print(json.dumps({'app': str(app), 'version': info['CFBundleShortVersionString'],
                          'mach_o_files': len(binaries), 'output': str(output),
                          'uploads_to_apple': False}, indent=2))
        return
    identities = run('security', 'find-identity', '-v', '-p', 'codesigning')
    if f'"{args.identity}"' not in identities:
        parser.error('Developer ID Application identity is not available in the keychain')
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='easycat-notarize-') as temporary:
        work = Path(temporary)
        stage = work / 'image'
        stage.mkdir()
        signed = stage / app.name
        shutil.copytree(app, signed, symlinks=True)
        for binary in macho_files(signed):
            run('codesign', '--force', '--options', 'runtime', '--timestamp',
                '--sign', args.identity, binary)
        run('codesign', '--force', '--options', 'runtime', '--timestamp',
            '--sign', args.identity, signed)
        run('codesign', '--verify', '--deep', '--strict', signed)
        archive = work / 'easycat.zip'
        run('ditto', '-c', '-k', '--keepParent', signed, archive)
        notarize(archive, args.keychain_profile,
                 output.with_name(output.name + '.app-notary-log.json'))
        run('xcrun', 'stapler', 'staple', signed)
        run('xcrun', 'stapler', 'validate', signed)
        run('spctl', '--assess', '--type', 'execute', '--verbose=2', signed)
        (stage / 'Applications').symlink_to('/Applications')
        run('hdiutil', 'create', '-volname', 'easycat', '-srcfolder', stage,
            '-format', 'UDZO', output)
    run('codesign', '--timestamp', '--sign', args.identity, output)
    notarize(output, args.keychain_profile, output.with_name(output.name + '.notary-log.json'))
    run('xcrun', 'stapler', 'staple', output)
    run('xcrun', 'stapler', 'validate', output)
    run('spctl', '--assess', '--type', 'open', '--context', 'context:primary-signature',
        '--verbose=2', output)
    run('hdiutil', 'verify', output)
    checksum = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix('.dmg.sha256').write_text(f'{checksum}  {output.name}\n')
    print(json.dumps({'dmg': str(output), 'sha256': checksum}, indent=2))


if __name__ == '__main__':
    main()
