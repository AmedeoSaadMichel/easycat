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
