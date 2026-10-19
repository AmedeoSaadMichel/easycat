#!/usr/bin/env python3
"""Build pinned Intel media libraries while reusing the runner's build tools."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
RECORDS = json.loads((ROOT / 'distribution/compliance/macos-sources.json').read_text())
COMPONENTS = {r['component'].rsplit('-', 1)[0]: r['component'].rsplit('-', 1)[1] for r in RECORDS}
PREFIX = Path('/usr/local')
# Homebrew denies reads from the runner's home while compiling formulae.
TOOLS = PREFIX / 'Cellar/easycat-intel-build-tools/1'


def run(args, **kwargs):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, **kwargs)


def environment():
    env = os.environ.copy()
    bins = [TOOLS / 'bin', *(PREFIX / 'opt' / name / 'bin' for name in COMPONENTS)]
    env['PATH'] = os.pathsep.join(map(str, bins)) + os.pathsep + env['PATH']
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
    run(['brew', 'install', 'nasm'], env=environment())
    tap = PREFIX / 'Homebrew/Library/Taps/easycat/homebrew-intel/Formula'
    tap.mkdir(parents=True, exist_ok=True)
    for recipe in (ROOT / 'distribution/compliance/intel-formulae').glob('*.rb'):
        shutil.copy2(recipe, tap / recipe.name)
    # Trust only these checked-in recipes on the disposable CI runner.
    run(['brew', 'trust', '--formula', *('easycat/intel/' + name for name in COMPONENTS)])
    # Load every recipe before spending time compiling any of them.
    formulae = json.loads(subprocess.check_output(
        ['brew', 'info', '--json=v2', *('easycat/intel/' + name for name in COMPONENTS)],
        env=environment(), text=True))['formulae']
    if {formula['name'] for formula in formulae} != set(COMPONENTS):
        raise RuntimeError('Pinned Intel formula preflight returned an incomplete set')
    for formula in formulae:
        version = formula['versions']['stable']
        if formula['revision']:
            version += '_' + str(formula['revision'])
        if COMPONENTS[formula['name']] != version:
            raise RuntimeError(f"Unexpected pinned version for {formula['name']}: {version}")
    print('Validated all pinned Intel formula definitions', flush=True)
    run([sys.executable, '-m', 'venv', TOOLS])
    run([TOOLS / 'bin/python', '-m', 'pip', 'install', 'meson==1.12.1'])
    # Homebrew helpers also resolve build tools through opt; Meson uses hosted Python.
    meson_opt = PREFIX / 'opt/meson'
    if meson_opt.is_symlink():
        meson_opt.unlink()
    if not meson_opt.exists():
        meson_opt.symlink_to(TOOLS)
    env = environment()
    versions = {}
    for tool in ('cmake', 'ninja', 'pkg-config', 'meson', 'nasm'):
        versions[tool] = subprocess.check_output([tool, '--version'], env=env, text=True).strip()
        print(tool, versions[tool], flush=True)
    output = ROOT / 'target/macos-verification'
    output.mkdir(parents=True, exist_ok=True)
    (output / 'build-tools.json').write_text(json.dumps(versions, indent=2) + '\n')
    if os.environ.get('GITHUB_PATH'):
        with open(os.environ['GITHUB_PATH'], 'a') as f:
            for directory in [TOOLS / 'bin', *(PREFIX / 'opt' / name / 'bin' for name in COMPONENTS)]:
                f.write(str(directory) + '\n')
        with open(os.environ['GITHUB_ENV'], 'a') as f:
            for key in ('PKG_CONFIG_PATH', 'PKG_CONFIG_LIBDIR'):
                f.write(key + '=' + env[key] + '\n')


def install(name):
    if os.environ.get('GITHUB_ACTIONS') != 'true':
        raise RuntimeError('Intel media provisioning is restricted to disposable CI runners')
    version = COMPONENTS[name]
    keg = PREFIX / 'Cellar' / name / version
    env = environment()
    recipe_changed = False
    if name == 'ffmpeg':
        installed_recipe = keg / '.brew/ffmpeg.rb'
        pinned_recipe = ROOT / 'distribution/compliance/intel-formulae/ffmpeg.rb'
        recipe_changed = (not installed_recipe.is_file() or
                          installed_recipe.read_bytes() != pinned_recipe.read_bytes())
    if not (keg / 'INSTALL_RECEIPT.json').is_file() or recipe_changed:
        # Only the disposable CI runner may replace its preinstalled core formula.
        if (PREFIX / 'Cellar' / name).exists():
            run(['brew', 'uninstall', '--force', '--ignore-dependencies', name], env=env)
        run(['brew', 'install', '--ignore-dependencies', 'easycat/intel/' + name], env=env)
    if not (keg / 'INSTALL_RECEIPT.json').is_file():
        raise RuntimeError(f'Expected pinned media package is missing: {keg}')
    opt = PREFIX / 'opt' / name
    linked = PREFIX / 'var/homebrew/linked' / name
    if name != 'openssl@4' and linked.is_symlink() and linked.resolve() != keg:
        old_keg = linked.resolve(strict=True)
        # brew unlink selects opt first: restore the old opt before unlinking it.
        if opt.is_symlink():
            opt.unlink()
        opt.symlink_to(old_keg)
        run(['brew', 'unlink', name], env=env)
    if opt.is_symlink():
        opt.unlink()
    opt.symlink_to(keg)
    if name != 'openssl@4' and (not linked.is_symlink() or linked.resolve() != keg):
        run(['brew', 'link', '--overwrite', name], env=env)
    # Direct linker probes as well as pkg-config must select the pinned keg.
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
