"""Release-tool tests use synthetic headers, never substitute them for runnable releases."""
import json
import struct
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
import release
import bottle


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "Cargo.toml").write_text('[package]\nname = "geditor"\nversion = "0.3.0"\n')
        (self.root / "README.md").write_text("# Test release fixture\n")
        (self.root / "LICENSE").write_text("MIT\n")
        self.dist = self.root / "dist"

    def tearDown(self):
        self.temp.cleanup()

    def fixture(self, target):
        os_name, arch, _ = release.TARGETS[target]
        data = bytearray(128)
        if os_name == "macos":
            data[:4] = b"\xcf\xfa\xed\xfe"
            struct.pack_into("<I", data, 4, 0x0100000C)
        elif os_name == "linux":
            data[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<H", data, 18, 183 if arch == "arm64" else 62)
        else:
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 60, 64)
            data[64:68] = b"PE\0\0"
            struct.pack_into("<H", data, 68, 0xAA64 if arch == "arm64" else 0x8664)
        path = self.root / (target + ".header-fixture")
        path.write_bytes(data)
        return path

    def pack(self, target):
        return release.package(target, self.fixture(target), self.dist, self.root)

    def test_tag_matches_package_version(self):
        self.assertEqual(release.check_tag("v0.3.0", self.root), "v0.3.0")
        with self.assertRaises(ValueError):
            release.check_tag("v9.0.0", self.root)

    def test_wrong_architecture_is_rejected(self):
        binary = self.fixture("aarch64-apple-darwin")
        with self.assertRaises(ValueError):
            release.verify_binary(binary, "aarch64-pc-windows-msvc")
        binary = self.fixture("x86_64-unknown-linux-gnu")
        with self.assertRaises(ValueError):
            release.verify_binary(binary, "aarch64-unknown-linux-gnu")

    def test_unix_archive_keeps_executable_mode_and_alias(self):
        manifest = self.pack("aarch64-apple-darwin")
        with tarfile.open(self.dist / manifest["archive"]) as archive:
            exe = next(item for item in archive if item.name.endswith("/geditor"))
            self.assertEqual(exe.mode & 0o777, 0o755)
            alias = next(item for item in archive if item.name.endswith("/ge"))
            self.assertTrue(alias.issym())
            self.assertEqual(alias.linkname, "geditor")

    def test_windows_archive_has_both_executable_names(self):
        manifest = self.pack("aarch64-pc-windows-msvc")
        with zipfile.ZipFile(self.dist / manifest["archive"]) as archive:
            names = {Path(name).name for name in archive.namelist()}
            self.assertIn("geditor.exe", names)
            self.assertIn("ge.exe", names)

    def test_finalize_requires_all_platforms_and_real_matching_hashes(self):
        self.pack("aarch64-apple-darwin")
        with self.assertRaises(ValueError):
            release.finalize(self.dist, "example/geditor")
        for target in release.TARGETS:
            if target != "aarch64-apple-darwin":
                self.pack(target)
        records = release.finalize(self.dist, "example/geditor")
        formula = (self.dist / "geditor.rb").read_text()
        for record in records:
            if record["os"] != "windows":
                self.assertIn(record["sha256"], formula)
        self.assertIn('bin.install_symlink "geditor" => "ge"', formula)
        with self.assertRaises(ValueError):
            release.formula(records, 'invalid/repo"injection')
        record = records[0]
        (self.dist / record["archive"]).write_bytes(b"modified after packaging")
        with self.assertRaises(ValueError):
            release.finalize(self.dist, "example/geditor")

    def test_bottles_preserve_binary_and_have_install_receipts(self):
        for target in release.TARGETS:
            self.pack(target)
        bottles = bottle.build(self.dist, "example/geditor")
        self.assertEqual(len(bottles), 3)
        formula = (self.dist / "geditor-bottled.rb").read_text()
        for tag, path, digest in bottles:
            self.assertIn(digest, formula)
            with tarfile.open(path) as archive:
                exe = archive.getmember("geditor/0.3.0/bin/geditor")
                self.assertEqual(exe.mode, 0o755)
                target = next(t for t, v in bottle.TAGS.items() if v == tag)
                self.assertEqual(archive.extractfile(exe).read(), self.fixture(target).read_bytes())
                alias = archive.getmember("geditor/0.3.0/bin/ge")
                self.assertTrue(alias.issym())
                self.assertEqual(alias.linkname, "geditor")
                receipt = json.load(archive.extractfile("geditor/0.3.0/INSTALL_RECEIPT.json"))
                self.assertEqual(receipt["source"]["versions"]["stable"], "0.3.0")
        before = {tag: digest for tag, _, digest in bottles}
        self.assertEqual(before, {tag: digest for tag, _, digest in bottle.build(self.dist, "example/geditor")})


if __name__ == "__main__":
    unittest.main()
