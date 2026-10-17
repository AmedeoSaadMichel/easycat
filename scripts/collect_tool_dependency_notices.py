#!/usr/bin/env python3
"""Preserve notices from the actual pinned third-party source archives for installers."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

from third_party_notices import is_notice_path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ROOT / 'dist/third-party-sources'
OUT = ROOT / 'distribution/tool-dependency-notices'


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    notices = OUT / 'notices'
    notices.mkdir(exist_ok=True)
    groups = [('macos', json.loads((SOURCES / 'macos-source-results.json').read_text())),
              ('portable', json.loads((SOURCES / 'portable-source-results.json').read_text())),
              ('portable-rust', json.loads((SOURCES / 'ffmpeg-rust-vendor-results.json').read_text()))]
    entries = []
    for platform, records in groups:
        for record in records:
            archive = SOURCES / record['filename']
            expected = record.get('sha256_collected', record.get('sha256'))
            checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
            if checksum != expected:
                raise RuntimeError('Source checksum mismatch: ' + archive.name)
            component = record.get('component', record.get('recipe', archive.stem))
            short = re.sub('[^a-zA-Z0-9_-]', '_', platform + '_' + component)[-55:]
            found = []
            with tarfile.open(archive, mode='r|*') as source:
                for member in source:
                    path = Path(member.name)
                    if not member.isfile() or not is_notice_path(path):
                        continue
                    content = source.extractfile(member).read()
                    key = hashlib.sha256((archive.name + ':' + member.name).encode()).hexdigest()[:16]
                    filename = short + '--' + key + '.txt'
                    (notices / filename).write_bytes(content)
                    found.append({'original_path': member.name, 'file': 'notices/' + filename,
                                  'sha256': hashlib.sha256(content).hexdigest()})
            if not found:
                # Some header-only SDK interfaces keep the complete license in comments.
                with tarfile.open(archive, mode='r|*') as source:
                    for member in source:
                        if not member.isfile() or Path(member.name).suffix.lower() not in ('.h', '.c', '.cpp'):
                            continue
                        prefix = source.extractfile(member).read(65536)
                        blocks = [block for block in re.findall(rb'/\*.*?\*/', prefix, re.S)
                                  if b'copyright' in block.lower() and any(term in block.lower() for term in (b'permission', b'redistribution', b'license'))]
                        if not blocks:
                            continue
                        content = b'\n\n'.join(blocks) + b'\n'
                        key = hashlib.sha256((archive.name + ':' + member.name).encode()).hexdigest()[:16]
                        filename = short + '--' + key + '.txt'
                        (notices / filename).write_bytes(content)
                        found.append({'original_path': member.name, 'file': 'notices/' + filename,
                                      'sha256': hashlib.sha256(content).hexdigest(), 'extracted_original_license_comment': True})
            if not found:
                raise RuntimeError('No upstream notices found: ' + archive.name)
            entries.append({'platform_source_set': platform, 'component': component,
                            'source_archive': archive.name, 'source_sha256': checksum, 'notices': found})
    (OUT / 'index.json').write_text(json.dumps(entries, indent=2) + '\n')
    (OUT / 'README.md').write_text('''# Tool dependency notices

These verbatim upstream notices accompany the pinned FFmpeg dependency source sets, including their static dependencies and Rust vendor sources. The index records the exact source archive hash and original path of each notice. The source sets are a conservative superset across platforms; they do not all apply to every executable, and none grants rights to unrelated easycat code. Root FFmpeg GPLv3 licensing and each component's own license remain intact.

Notice filenames are shortened to remain usable when extracting installers on Windows. Contents are preserved verbatim. Full original sources and build recipes accompany the release separately.
''')
    print(f'Preserved {sum(len(entry["notices"]) for entry in entries)} upstream notices from {len(entries)} pinned source archives')


if __name__ == '__main__':
    main()
