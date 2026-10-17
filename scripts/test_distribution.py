"""Regression checks for private-code exclusion and source integrity."""
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import prepare_public_distribution as public_distribution

from collect_distribution_sources import collect_crates, collect_mac
from third_party_notices import prepare_notices
from resolve_crate_license_cases import verify_declaration
from audit_distribution_linkage import reject_ffmpeg_imports
from assemble_release_verification import require_passed


class DistributionTests(unittest.TestCase):
    def test_notices_exclude_workspace_source_and_symlinks(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            crate = root / 'registry/public-1.0'
            crate.mkdir(parents=True)
            (crate / 'Cargo.toml').write_text('fixture')
            (crate / 'LICENSE').write_text('Public upstream notice')
            (crate / 'copying.rs').write_text('Third-party implementation, not a license notice')
            private = root / 'easycat-private.rs'
            private.write_text('PRIVATE_EASYCAT_SOURCE_SENTINEL')
            (crate / 'LICENSE-link').symlink_to(private)
            records = prepare_notices(root / 'notices', {'packages': [
                {'name': 'easycat-private', 'source': None, 'manifest_path': str(private)},
                {'name': 'public', 'version': '1.0', 'source': 'registry+https://github.com/rust-lang/crates.io-index',
                 'manifest_path': str(crate / 'Cargo.toml'), 'license': 'MIT'},
            ]})
            self.assertEqual([item['name'] for item in records], ['public'])
            self.assertEqual(records[0]['notice_files'], ['LICENSE'])
            self.assertGreaterEqual((root / 'notices/rust-licenses/public-1.0/LICENSE').stat().st_mtime, 315532800)
            for path in (root / 'notices').rglob('*'):
                if path.is_file():
                    self.assertNotIn(b'PRIVATE_EASYCAT_SOURCE_SENTINEL', path.read_bytes())

    def test_embedded_font_license_texts_are_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            crate = root / 'public'
            (crate / 'fonts').mkdir(parents=True)
            (crate / 'Cargo.toml').write_text('fixture')
            for name in ['OFL.txt', 'UFL.txt', 'Hack-Regular.txt', 'emoji-icon-font-mit-license.txt']:
                (crate / 'fonts' / name).write_text('Original font license ' + name)
            (crate / 'fonts/Ubuntu-Light.ttf').write_bytes(b'FONT_BINARY')
            records = prepare_notices(root / 'notices', {'packages': [{
                'source': 'registry+https://github.com/rust-lang/crates.io-index',
                'name': 'font-package', 'version': '1.0', 'license': 'MIT AND OFL-1.1',
                'manifest_path': str(crate / 'Cargo.toml'),
            }]})
            self.assertEqual(len(records[0]['notice_files']), 4)
            self.assertTrue((root / 'notices/rust-licenses/font-package-1.0/fonts/UFL.txt').is_file())
            self.assertFalse((root / 'notices/rust-licenses/font-package-1.0/fonts/Ubuntu-Light.ttf').exists())

    def test_private_registry_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(RuntimeError, 'private or unreviewed registry'):
                prepare_notices(Path(temporary) / 'notices', {'packages': [
                    {'source': 'registry+https://private.example/index'},
                ]})

    def test_crate_source_export_excludes_workspace(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'registry/src/crates.io/public-1.0'
            cache = root / 'registry/cache/crates.io'
            source.mkdir(parents=True)
            cache.mkdir(parents=True)
            (cache / 'public-1.0.crate').write_bytes(b'PUBLIC_CRATE_ARCHIVE')
            output = root / 'out'
            output.mkdir()
            collect_crates({'packages': [
                {'source': None, 'name': 'easycat-private'},
                {'name': 'public', 'version': '1.0', 'source': 'registry+https://github.com/rust-lang/crates.io-index',
                 'manifest_path': str(source / 'Cargo.toml'), 'license': 'MIT'},
            ]}, output)
            with tarfile.open(output / 'rust-third-party-sources.tar.gz') as archive:
                self.assertEqual(archive.getnames(), ['crates/public-1.0.crate'])
            index = json.loads((output / 'rust-third-party-index.json').read_text())
            self.assertEqual(len(index), 1)

    def test_public_source_archive_uses_an_explicit_allowlist(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            public = root / 'distribution'
            public.mkdir()
            (public / 'notice.txt').write_text('Public notice')
            (public / 'private.rs').write_text('PRIVATE_EASYCAT_SOURCE_SENTINEL')
            (public / 'public-files.json').write_text('["notice.txt"]')
            archive_path = root / 'source.tar.gz'
            with patch.object(public_distribution, 'ROOT', root):
                public_distribution.bundle_sources(archive_path, [])
            with tarfile.open(archive_path) as archive:
                self.assertEqual(archive.getnames(), ['compliance-and-notices/notice.txt'])

    def test_upstream_license_declaration_must_match_exact_package(self):
        manifest = '[package]\nname = "block"\nversion = "0.1.6"\nlicense = "MIT"\nauthors = ["Steven Sheldon"]\n'
        self.assertEqual(verify_declaration(manifest, 'block', '0.1.6', 'MIT'), ['Steven Sheldon'])
        with self.assertRaisesRegex(ValueError, 'license'):
            verify_declaration(manifest.replace('"MIT"', '"GPL-3.0-only"'), 'block', '0.1.6', 'MIT')
        with self.assertRaisesRegex(ValueError, 'version'):
            verify_declaration(manifest, 'block', '0.2.0', 'MIT')

    def test_modified_manifest_cannot_reuse_a_resolved_license(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / 'Cargo.toml'
            manifest.write_text('[package]\nname = "block"\nlicense = "MIT"\n# modified\n')
            with self.assertRaisesRegex(RuntimeError, 'evidence does not match'):
                prepare_notices(root / 'notices', {'packages': [{
                    'source': 'registry+https://github.com/rust-lang/crates.io-index',
                    'name': 'block', 'version': '0.1.6', 'license': 'MIT', 'manifest_path': str(manifest),
                }]})

    def test_direct_ffmpeg_linkage_blocks_private_app_release(self):
        reject_ffmpeg_imports(['KERNEL32.dll', 'libc.so.6', '/usr/lib/libSystem.B.dylib'])
        for name in ['avcodec-63.dll', 'libavformat.so.63', '/app/lib/libavutil.61.dylib']:
            with self.assertRaisesRegex(RuntimeError, 'directly links'):
                reject_ffmpeg_imports([name])

    def test_native_readiness_rejects_incomplete_checks(self):
        require_passed({'status': 'passed'})
        for report in [{'status': 'running'}, {'status': 'failed'}, {}]:
            with self.assertRaisesRegex(ValueError, 'has not passed'):
                require_passed(report)

    def test_wrong_source_checksum_stays_blocked(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'fixture-src.tar.gz').write_bytes(b'WRONG_SOURCE')
            result = collect_mac({'component': 'fixture', 'url': 'https://example.invalid/src.tar.gz',
                                  'git_revision': None, 'sha256': '0' * 64}, root)
            self.assertEqual(result['status'], 'blocked')
            self.assertNotIn('sha256_collected', result)


if __name__ == '__main__':
    unittest.main()
