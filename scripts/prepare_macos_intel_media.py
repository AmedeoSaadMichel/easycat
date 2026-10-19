#!/usr/bin/env python3
"""Build pinned Intel media libraries while reusing the runner's build tools."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
RECORDS = json.loads((ROOT / 'distribution/compliance/macos-sources.json').read_text())
COMPONENTS = {r['component'].rsplit('-', 1)[0]: r['component'].rsplit('-', 1)[1] for r in RECORDS}
PREFIX = Path('/usr/local')
TOOLS = ROOT / 'target/intel-build-tools'


def run(args, **kwargs):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, **kwargs)


def environment():
    env = os.environ.copy()
    env['PATH'] = str(TOOLS / 'bin') + os.pathsep + env['PATH']
    env['HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK'] = '1'
    env['HOMEBREW_NO_INSTALL_CLEANUP'] = '1'
    env.pop('HOMEBREW_NO_INSTALL_UPGRADE', None)
    pkg = [str(PREFIX / 'opt' / name / suffix) for name in COMPONENTS
           for suffix in ('lib/pkgconfig', 'share/pkgconfig')]
    env['PKG_CONFIG_PATH'] = os.pathsep.join(pkg)
    env['PKG_CONFIG_LIBDIR'] = env['PKG_CONFIG_PATH']
    return env


def tools():
    assert os.uname().machine == 'x86_64'
    run(['brew', 'update'])
    run([sys.executable, '-m', 'venv', TOOLS])
    run([TOOLS / 'bin/python', '-m', 'pip', 'install', 'meson==1.12.1'])
    # Superenv resolves declared build tools through opt; Meson runs on hosted Python.
    meson_opt = PREFIX / 'opt/meson'
    if meson_opt.is_symlink():
        meson_opt.unlink()
    if not meson_opt.exists():
        meson_opt.symlink_to(TOOLS)
    env = environment()
    versions = {}
    for tool in ('cmake', 'ninja', 'pkg-config', 'meson'):
        versions[tool] = subprocess.check_output([tool, '--version'], env=env, text=True).strip()
        print(tool, versions[tool], flush=True)
    output = ROOT / 'target/macos-verification'
    output.mkdir(parents=True, exist_ok=True)
    (output / 'build-tools.json').write_text(json.dumps(versions, indent=2) + '\n')
    if os.environ.get('GITHUB_PATH'):
        with open(os.environ['GITHUB_PATH'], 'a') as f:
            f.write(str(TOOLS / 'bin') + '\n')
        with open(os.environ['GITHUB_ENV'], 'a') as f:
            for key in ('PKG_CONFIG_PATH', 'PKG_CONFIG_LIBDIR'):
                f.write(key + '=' + env[key] + '\n')


def install(name):
    version = COMPONENTS[name]
    keg = PREFIX / 'Cellar' / name / version
    env = environment()
    if not (keg / 'INSTALL_RECEIPT.json').is_file():
        # Runtime versions stay pinned; existing CMake/Python avoid costly tool upgrades.
        run(['brew', 'install', '--ignore-dependencies', name], env=env)
    if not (keg / 'INSTALL_RECEIPT.json').is_file():
        raise RuntimeError(f'Expected pinned media package is missing: {keg}')
    opt = PREFIX / 'opt' / name
    if opt.is_symlink():
        opt.unlink()
    opt.symlink_to(keg)
    if name != 'openssl@4':
        run(['brew', 'unlink', name], env=env)
        run(['brew', 'link', '--overwrite', name], env=env)
    print('Verified installed version:', name, version, flush=True)


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('packages', nargs='*')
    parser.add_argument('--tools', action='store_true')
    args = parser.parse_args(argv)
    unknown = set(args.packages) - set(COMPONENTS)
    if unknown:
        parser.error('Unknown media packages: ' + ', '.join(sorted(unknown)))
    return args


if __name__ == '__main__':
    args = parse_args()
    if args.tools:
        tools()
    for package in args.packages:
        install(package)
