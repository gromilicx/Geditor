#!/usr/bin/env python3
"""Build-independent release packaging and Homebrew generation (Python stdlib only)."""
import argparse
import hashlib
import json
import re
import shutil
import struct
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    "aarch64-apple-darwin": ("macos", "arm64", "macOS 11 or newer"),
    "x86_64-unknown-linux-gnu": ("linux", "x64", "glibc 2.35 or newer (Ubuntu 22.04+)"),
    "aarch64-unknown-linux-gnu": ("linux", "arm64", "glibc 2.35 or newer (Ubuntu 22.04+)"),
    "x86_64-pc-windows-msvc": ("windows", "x64", "Windows 10 or newer; Windows Terminal recommended"),
    "aarch64-pc-windows-msvc": ("windows", "arm64", "Windows 11 ARM64; Windows Terminal recommended"),
}


def version(root=ROOT):
    text = (root / "Cargo.toml").read_text(encoding="utf-8")
    package = text.split("[package]", 1)[1].split("\n[", 1)[0]
    found = re.search(r'^version\s*=\s*"([^"]+)"', package, re.M)
    if not found or not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", found[1]):
        raise ValueError("Cargo.toml has no supported semantic package version")
    return found[1]


def check_tag(tag, root=ROOT):
    expected = "v" + version(root)
    if tag and tag != expected:
        raise ValueError("Release tag must be %s, got %s" % (expected, tag))
    return expected


def verify_binary(path, target):
    """Reject mislabeled files, even when packaging on another host."""
    data = path.read_bytes()
    os_name, arch, _ = TARGETS[target]
    if os_name == "macos":
        if len(data) < 8 or data[:4] != b"\xcf\xfa\xed\xfe":
            raise ValueError("Expected a 64-bit little-endian Mach-O binary")
        valid = struct.unpack_from("<I", data, 4)[0] == 0x0100000C
    elif os_name == "linux":
        if len(data) < 20 or data[:6] != b"\x7fELF\x02\x01":
            raise ValueError("Expected a 64-bit little-endian ELF binary")
        valid = struct.unpack_from("<H", data, 18)[0] == (183 if arch == "arm64" else 62)
    else:
        if len(data) < 64 or data[:2] != b"MZ":
            raise ValueError("Expected a Windows PE executable")
        offset = struct.unpack_from("<I", data, 60)[0]
        if len(data) < offset + 6 or data[offset:offset + 4] != b"PE\0\0":
            raise ValueError("Invalid PE signature")
        valid = struct.unpack_from("<H", data, offset + 4)[0] == (0xAA64 if arch == "arm64" else 0x8664)
    if not valid:
        raise ValueError("Executable architecture does not match " + target)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def package(target, binary, out, root=ROOT):
    verify_binary(binary, target)
    current = version(root)
    os_name, arch, minimum = TARGETS[target]
    stem = "geditor-%s-%s-%s" % (current, os_name, arch)
    suffix = ".zip" if os_name == "windows" else ".tar.gz"
    out.mkdir(parents=True, exist_ok=True)
    archive = out / (stem + suffix)
    with tempfile.TemporaryDirectory(prefix="geditor-release-") as temp:
        stage = Path(temp) / stem
        stage.mkdir()
        exe = "geditor.exe" if os_name == "windows" else "geditor"
        shutil.copy2(binary, stage / exe)
        (stage / exe).chmod(0o755)
        if os_name == "windows":
            shutil.copy2(stage / exe, stage / "ge.exe")
        else:
            (stage / "ge").symlink_to("geditor")
        for file in ["README.md", "LICENSE"] + (["LICENSE-MIT"] if (root / "LICENSE-MIT").exists() else []):
            shutil.copy2(root / file, stage / file)
        (stage / "RUN.txt").write_text(
            "GEditor %s\nTarget: %s\nRequires: %s\n\n"
            "Run in an interactive terminal: ge .\n"
            "Or open a file: ge README.md\n"
            "No Rust toolchain is needed to run this executable.\n"
            "For Windows, add the extracted folder to your user PATH.\n"
            "For macOS/Linux, copy geditor to ~/.local/bin and create ge -> geditor.\n"
            "Linux GNU builds do not run on Alpine/musl.\n" % (current, target, minimum), encoding="utf-8")
        if os_name == "windows":
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as output:
                for file in sorted(stage.iterdir()):
                    output.write(file, stem + "/" + file.name)
        else:
            with tarfile.open(archive, "w:gz", dereference=False) as output:
                output.add(stage, arcname=stem)
    manifest = {
        "version": current, "target": target, "os": os_name, "arch": arch,
        "archive": archive.name, "sha256": sha256(archive), "minimum": minimum,
    }
    (out / (target + ".manifest.json")).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def validate_manifest(record, directory):
    if record.get("target") not in TARGETS:
        raise ValueError("Unknown target in release manifest")
    current = record.get("version", "")
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", current):
        raise ValueError("Invalid release version")
    os_name, arch, _ = TARGETS[record["target"]]
    suffix = ".zip" if os_name == "windows" else ".tar.gz"
    expected = "geditor-%s-%s-%s%s" % (current, os_name, arch, suffix)
    if record.get("archive") != expected or record.get("os") != os_name or record.get("arch") != arch:
        raise ValueError("Archive identity does not match target")
    if sha256(directory / expected) != record.get("sha256"):
        raise ValueError("Archive checksum mismatch: " + expected)


def formula(records, repository):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*", repository):
        raise ValueError("Repository must be OWNER/REPOSITORY")
    indexed = {item["target"]: item for item in records}
    mac = indexed["aarch64-apple-darwin"]
    linux_x64 = indexed["x86_64-unknown-linux-gnu"]
    linux_arm = indexed["aarch64-unknown-linux-gnu"]
    current = mac["version"]
    if any(item["version"] != current for item in records):
        raise ValueError("Release manifests contain different versions")
    url = "https://github.com/%s/releases/download/v%s/" % (repository, current)
    return '''# Generated from verified release archives. Copy to homebrew-tap/Formula/geditor.rb.
class Geditor < Formula
  desc "Terminal code editor with Markdown preview, AI chat, and an integrated shell"
  homepage "https://github.com/%(repository)s"
  version "%(version)s"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    depends_on arch: :arm64
    depends_on macos: :big_sur
    url "%(url)s%(mac_archive)s"
    sha256 "%(mac_sha)s"
  end

  on_linux do
    if Hardware::CPU.arm?
      url "%(url)s%(arm_archive)s"
      sha256 "%(arm_sha)s"
    else
      url "%(url)s%(x64_archive)s"
      sha256 "%(x64_sha)s"
    end
  end

  def install
    bin.install "geditor"
    bin.install_symlink "geditor" => "ge"
  end

  test do
    assert_match "GEditor #{version}", shell_output("#{bin}/geditor --version")
    assert_match "Usage:", shell_output("#{bin}/ge --help")
  end
end
''' % {
        "repository": repository, "version": current, "url": url,
        "mac_archive": mac["archive"], "mac_sha": mac["sha256"],
        "x64_archive": linux_x64["archive"], "x64_sha": linux_x64["sha256"],
        "arm_archive": linux_arm["archive"], "arm_sha": linux_arm["sha256"],
    }


def finalize(directory, repository):
    records = []
    for file in sorted(directory.glob("*.manifest.json")):
        record = json.loads(file.read_text(encoding="utf-8"))
        validate_manifest(record, directory)
        records.append(record)
    if len(records) != len(TARGETS) or {r["target"] for r in records} != set(TARGETS):
        raise ValueError("All five distinct native platform builds must succeed before finalizing")
    (directory / "geditor.rb").write_text(formula(records, repository), encoding="utf-8")
    files = sorted([directory / record["archive"] for record in records] + [directory / "geditor.rb"])
    (directory / "SHA256SUMS").write_text("".join("%s  %s\n" % (sha256(file), file.name) for file in files), encoding="utf-8")
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check")
    check.add_argument("--tag", default="")
    build = commands.add_parser("package")
    build.add_argument("--target", choices=TARGETS, required=True)
    build.add_argument("--binary", type=Path, required=True)
    build.add_argument("--out", type=Path, default=ROOT / "dist")
    finish = commands.add_parser("finalize")
    finish.add_argument("--directory", type=Path, required=True)
    finish.add_argument("--repository", required=True)
    args = parser.parse_args()
    if args.command == "check":
        print(check_tag(args.tag))
    elif args.command == "package":
        print(json.dumps(package(args.target, args.binary, args.out), indent=2))
    else:
        print("Verified %d builds; generated geditor.rb and SHA256SUMS" % len(finalize(args.directory, args.repository)))


if __name__ == "__main__":
    main()
