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
    env = environment()
    for tool in ('cmake', 'ninja', 'pkg-config', 'meson'):
        run([tool, '--version'], env=env)
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
        installed = subprocess.run(['brew', 'list', '--versions', name], capture_output=True, text=True)
        operation = 'upgrade' if installed.returncode == 0 and installed.stdout.strip() else 'install'
        # Runtime versions stay pinned; existing CMake/Python avoid costly tool upgrades.
        run(['brew', operation, '--ignore-dependencies', '--env=std', name], env=env)
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


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('packages', nargs='*', choices=list(COMPONENTS))
    parser.add_argument('--tools', action='store_true')
    args = parser.parse_args()
    if args.tools:
        tools()
    for package in args.packages:
        install(package)
