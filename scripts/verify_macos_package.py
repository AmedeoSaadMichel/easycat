#!/usr/bin/env python3
"""Verify native macOS package exports and startup using generated media only."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    app = ROOT / 'target/macos-package/easycat.app'
    contents = app / 'Contents'
    helpers = contents / 'Helpers'
    binary = contents / 'MacOS/easycat-bin'
    output = ROOT / 'target/macos-verification'
    output.mkdir(parents=True, exist_ok=True)
    info = json.loads((contents / 'Resources/build-info.json').read_text())
    assert info['architecture'] == platform.machine() == 'x86_64'
    data = binary.read_bytes()
    assert info['application_version'].encode() in data
    assert b'--simulate-update=required' not in data
    assert b'--simulate-update=optional' not in data
    assert (contents / 'Resources/Licenses/easycat-MIT.txt').read_bytes() == (ROOT / 'LICENSE').read_bytes()
    env = dict(os.environ, PATH=str(helpers) + ':/usr/bin:/bin:/usr/sbin:/sbin',
               SDL_AUDIODRIVER='dummy', SDL_VIDEODRIVER='dummy')

    def run(args):
        return subprocess.check_output(list(map(str, args)), env=env, text=True,
                                       stderr=subprocess.STDOUT, timeout=180)

    run(['codesign', '--verify', '--deep', '--strict', app])
    imports = run(['otool', '-L', binary])
    assert not any(name in imports for name in ['libavcodec', 'libavformat', 'libavutil'])
    with tempfile.TemporaryDirectory(prefix='easycat-intel-smoke-') as temporary:
        work = Path(temporary)
        source = work / 'source.mp4'
        run([helpers / 'ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=160x120:rate=10',
             '-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo', '-t', '1', '-c:v', 'libx264', '-c:a', 'aac', source])
        cli = ROOT / 'target/release/easycat'
        run([cli, 'cut', source, work / 'trim.mp4', '--start', '0', '--end', '0.5'])
        run([cli, 'remux', source, work / 'remux.mkv'])
        run([cli, 'compress', source, work / 'compressed.mp4', '--software', '--height', '96'])
        for name in ['trim.mp4', 'remux.mkv', 'compressed.mp4']:
            probe = json.loads(run([helpers / 'ffprobe', '-v', 'error', '-show_streams', '-of', 'json', work / name]))
            assert any(s['codec_type'] == 'video' for s in probe['streams'])
        gui_env = env.copy()
        gui_env.pop('SDL_VIDEODRIVER', None)
        with (output / 'gui.log').open('w') as log:
            process = subprocess.Popen([str(contents / 'MacOS/easycat'), str(source)],
                                       cwd=work, env=gui_env, stdout=log, stderr=log)
            try:
                for _ in range(10):
                    time.sleep(1)
                    if process.poll() is not None:
                        raise RuntimeError('Packaged GUI exited; see target/macos-verification/gui.log')
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
    report = dict(info, status='passed', app_sha256=hashlib.sha256(data).hexdigest(),
                  cli_exports='passed', native_gui_startup='passed', strict_codesign='passed',
                  bundled_codec_tests='see codecs/result.json', direct_ffmpeg_library_linkage=False)
    (output / 'package.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Native Intel package exports, GUI startup, version and signature checks passed')


if __name__ == '__main__':
    main()
