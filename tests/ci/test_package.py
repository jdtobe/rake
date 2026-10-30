from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
VERIFY = ROOT / 'scripts/verify-package.sh'


class PackageValidation(unittest.TestCase):
    def test_invalid_versions_do_not_touch_existing_bundle(self):
        with tempfile.TemporaryDirectory(prefix='rake-package-') as directory:
            root = Path(directory)
            (root / 'scripts').mkdir()
            shutil.copy2(ROOT / 'scripts/package.sh', root / 'scripts/package.sh')
            (root / 'Cargo.toml').write_text('[package]\nversion = "0.1.0"\n')
            (root / 'dist/Rake.app').mkdir(parents=True)
            sentinel = root / 'dist/Rake.app/keep'
            sentinel.write_text('previous preview')
            for version in ['../escape', 'v26.10.0', '26.13.0', '26.10.01', '1.2', '26.10.0\nextra']:
                result = subprocess.run(['sh', str(root / 'scripts/package.sh'), version],
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0, version)
                self.assertIn('invalid version', result.stderr, result.stderr)
                self.assertEqual(sentinel.read_text(), 'previous preview')

    def test_corrupt_image_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix='rake-corrupt-') as directory:
            image = Path(directory) / 'bad.dmg'
            image.write_bytes(b'not a disk image')
            result = subprocess.run(['sh', str(VERIFY), str(image), '0.1.0'],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn("No such file", result.stderr)


@unittest.skipUnless(sys.platform == 'darwin', 'native DMG checks require macOS')
class NativePackage(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        binary = ROOT / 'target/release/rake'
        if not binary.is_file():
            raise RuntimeError('run cargo build --release --locked before native package tests')
        cls.temp = tempfile.TemporaryDirectory(prefix='rake-native-package-')
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name)
        for relative in ['scripts/package.sh', 'assets/icons/Rake.icns', 'Cargo.toml']:
            destination = cls.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / relative, destination)
        (cls.root / 'target/release').mkdir(parents=True)
        shutil.copy2(binary, cls.root / 'target/release/rake')
        result = subprocess.run(['sh', str(cls.root / 'scripts/package.sh'), '0.1.0'],
                                cwd=cls.root, check=True, capture_output=True, text=True)
        cls.image = cls.root / result.stdout.strip()

    def verify(self, version):
        return subprocess.run(['sh', str(VERIFY), str(self.image), version],
                              capture_output=True, text=True)

    def test_real_image_is_verified_and_checksum_reported(self):
        result = self.verify('0.1.0')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stdout, r'^[0-9a-f]{64}  ')

    def test_wrong_expected_version_is_rejected(self):
        result = self.verify('26.10.1')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('version', result.stderr)


if __name__ == '__main__':
    unittest.main()
