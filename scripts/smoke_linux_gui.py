#!/usr/bin/env python3
"""Open the packaged Linux preview under Xvfb and capture its rendered window."""
import os
from pathlib import Path
import subprocess
import tempfile
import time

root = Path(__file__).resolve().parents[1]
app = root / 'target/easycat-0.1.0-linux-x86_64/easycat'
source = root / 'target/portable-smoke-linux/source.mp4'
with tempfile.TemporaryDirectory(prefix='easycat-xdg-') as runtime:
    env = os.environ.copy()
    env.update(XDG_RUNTIME_DIR=runtime, LIBGL_ALWAYS_SOFTWARE='1')
    env.pop('WAYLAND_DISPLAY', None)
    log = root / 'target/linux-gui.log'
    with log.open('w') as output:
        process = subprocess.Popen([str(app), str(source)], env=env, stdout=output, stderr=output, cwd=runtime)
        try:
            for _ in range(10):
                time.sleep(1)
                if process.poll() is not None:
                    raise RuntimeError(f'Packaged GUI exited unexpectedly: {log.read_text()}')
            subprocess.run(['scrot', str(root / 'target/linux-preview.png')], check=True)
            print('Packaged GUI stayed running and rendered under Xvfb.')
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
