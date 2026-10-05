"""Verify archive names, contents against the host contract."""
import importlib.util
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "package_plugin", Path(__file__).resolve().parents[2] / "scripts/package_plugin.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PackageTests(unittest.TestCase):
    def test_host_archive_contract_for_every_target(self):
        for target in module.TARGETS:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / 'Cargo.toml').write_text('[package]\nversion = "3.0.2"\n')
                manifest = 'name = "sqllog2db"\nversion = "3.0.2"\n'
                (root / 'dm-plugin.toml').write_text(manifest)
                binary = 'dm-sqllog2db' + ('.exe' if 'windows' in target else '')
                source = root / 'target' / target / 'release' / binary
                source.parent.mkdir(parents=True)
                source.write_bytes(b'compiled-plugin')
                archive = module.package(target, 'v3.0.2', root, root / 'dist')
                folder = 'dm-sqllog2db-v3.0.2-' + target
                expected = {folder + '/dm-plugin.toml', folder + '/' + binary}
                if binary.endswith('.exe'):
                    with zipfile.ZipFile(archive) as output:
                        self.assertEqual(set(output.namelist()), expected)
                        self.assertEqual(output.read(folder + '/' + binary), source.read_bytes())
                        self.assertEqual(output.read(folder + '/dm-plugin.toml'), manifest.encode())
                else:
                    with tarfile.open(archive) as output:
                        self.assertEqual(set(output.getnames()), expected)
                        entry = output.getmember(folder + '/' + binary)
                        self.assertEqual(entry.mode, 0o755)
                        self.assertEqual(output.extractfile(entry).read(), source.read_bytes())
                        self.assertEqual(output.extractfile(folder + '/dm-plugin.toml').read(), manifest.encode())
                self.assertEqual(list((root / 'dist').iterdir()), [archive])
                with self.assertRaises(ValueError):
                    module.package(target, 'v9.9.9', root, root / 'invalid')
                self.assertFalse((root / 'invalid').exists())
                (root / 'dm-plugin.toml').write_text('version = "3.0.1"\n')
                with self.assertRaises(ValueError):
                    module.package(target, 'v3.0.2', root, root / 'invalid')

    def test_unsupported_target(self):
        with self.assertRaises(ValueError):
            module.package('../invalid', 'v3.0.2')
