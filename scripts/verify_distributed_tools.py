#!/usr/bin/env python3
"""Exercise the actual bundled tools on generated media; never use user videos."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile


def command(args, env):
    result = subprocess.run([str(a) for a in args], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=180)
    if result.returncode:
        raise RuntimeError(result.stdout)
    return result.stdout


def verify(helpers, libraries, output):
    helpers = helpers.resolve()
    output.mkdir(parents=True, exist_ok=True)
    suffix = '.exe' if platform.system() == 'Windows' else ''
    env = dict(os.environ, PATH=str(helpers) + os.pathsep + os.environ.get('PATH', ''),
               LD_LIBRARY_PATH=str(libraries), SDL_AUDIODRIVER='dummy', SDL_VIDEODRIVER='dummy')
    ffmpeg, ffprobe, ffplay = [helpers / (tool + suffix) for tool in ('ffmpeg', 'ffprobe', 'ffplay')]
    config = command([ffmpeg, '-hide_banner', '-buildconf'], env)
    license_text = command([ffmpeg, '-hide_banner', '-L'], env)
    if '--enable-gpl' not in config or '--enable-version3' not in config or '--enable-nonfree' in config:
        raise RuntimeError('Unexpected FFmpeg license configuration')
    results = []
    with tempfile.TemporaryDirectory(prefix='easycat-native-codec-') as temporary:
        work = Path(temporary)
        codecs = [('libx264', 'h264', 'mp4', ['-preset', 'ultrafast', '-crf', '23']),
                  ('libx265', 'hevc', 'mp4', ['-preset', 'ultrafast', '-crf', '28', '-x265-params', 'pools=2']),
                  ('libvpx-vp9', 'vp9', 'webm', ['-deadline', 'realtime', '-crf', '33', '-b:v', '0']),
                  ('libsvtav1', 'av1', 'mp4', ['-preset', '12', '-crf', '38', '-svtav1-params', 'lp=2'])]
        for encoder, expected, extension, options in codecs:
            target = work / (encoder + '.' + extension)
            audio = 'libopus' if extension == 'webm' else 'aac'
            command([ffmpeg, '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=160x96:rate=10',
                     '-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo', '-t', '0.4', '-threads', '2',
                     '-c:v', encoder, *options, '-c:a', audio, target], env)
            probe = json.loads(command([ffprobe, '-v', 'error', '-show_streams', '-of', 'json', target], env))
            if probe['streams'][0]['codec_name'] != expected or not any(s['codec_type'] == 'audio' for s in probe['streams']):
                raise RuntimeError('Encoder output does not match requested profile')
            command([ffmpeg, '-v', 'error', '-i', target, '-f', 'null', '-'], env)
            if encoder == 'libx264':
                remux = work / 'remux.mkv'
                command([ffmpeg, '-v', 'error', '-i', target, '-map', '0', '-c', 'copy', remux], env)
                hashes = [command([ffmpeg, '-v', 'error', '-i', path, '-map', '0:v:0', '-c', 'copy', '-f', 'hash', '-hash', 'sha256', '-'], env).strip() for path in (target, remux)]
                if hashes[0] != hashes[1]:
                    raise RuntimeError('Remux changed encoded video payload')
                command([ffmpeg, '-v', 'error', '-i', target, '-frames:v', '1', '-vf', 'scale=80:-2', work / 'thumbnail.png'], env)
                command([ffplay, '-nodisp', '-autoexit', '-loglevel', 'error', target], env)
            results.append({'encoder': encoder, 'decoder_roundtrip': 'passed', 'audio': audio})
        ten = work / 'ten-bit.mp4'
        command([ffmpeg, '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=160x96:rate=10', '-t', '0.3',
                 '-pix_fmt', 'yuv420p10le', '-c:v', 'libx265', '-preset', 'ultrafast', '-x265-params', 'pools=2', ten], env)
        data = json.loads(command([ffprobe, '-v', 'error', '-show_streams', '-of', 'json', ten], env))
        if data['streams'][0]['pix_fmt'] != 'yuv420p10le':
            raise RuntimeError('10-bit HEVC support missing')
    report = {'status': 'passed', 'platform': platform.system(), 'codecs': results,
              'hevc_10bit': 'passed', 'lossless_remux': 'passed', 'thumbnail': 'passed', 'audio_playback': 'passed',
              'tool_sha256': {tool.name: hashlib.sha256(tool.read_bytes()).hexdigest() for tool in (ffmpeg, ffprobe, ffplay)},
              'scope': 'Actual distributed executable codec/export tests; headless audio, not interactive desktop playback'}
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    (output / 'buildconf.txt').write_text(config)
    (output / 'license.txt').write_text(license_text)
    print('Distributed codec, 10-bit, remux, thumbnail and audio tests passed on', platform.system())


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--helpers', type=Path, required=True)
    parser.add_argument('--libraries', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    verify(args.helpers, args.libraries, args.output)
