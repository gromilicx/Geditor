#!/usr/bin/env python3
"""Repackage verified native Unix releases as relocatable Homebrew bottles.

This never compiles or replaces executables. Supplemental bottle checksums are
separate from the original standalone release's SHA256SUMS.
"""
import argparse
import gzip
import io
import json
import tarfile
import tempfile
from pathlib import Path
import release

TAGS = {
    "aarch64-apple-darwin": "arm64_big_sur",
    "aarch64-unknown-linux-gnu": "arm64_linux",
    "x86_64-unknown-linux-gnu": "x86_64_linux",
}


def build(directory, repository):
    records = [json.loads(p.read_text(encoding="utf-8"))
               for p in sorted(directory.glob("*.manifest.json"))]
    if len(records) != 5 or {r["target"] for r in records} != set(release.TARGETS):
        raise ValueError("All five native release manifests are required")
    for record in records:
        release.validate_manifest(record, directory)
    source_formula = release.formula(records, repository)
    current = records[0]["version"]
    bottles = []
    for record in records:
        target = record["target"]
        if target not in TAGS:
            continue
        tag = TAGS[target]
        prefix = "geditor/" + current
        contents = {}
        with tarfile.open(directory / record["archive"], "r:gz") as original:
            stem = record["archive"].removesuffix(".tar.gz")
            for name in ("geditor", "README.md", "LICENSE", "LICENSE-MIT", "RUN.txt"):
                try:
                    item = original.getmember(stem + "/" + name)
                except KeyError:
                    if name == "LICENSE-MIT":
                        continue
                    raise
                if not item.isfile():
                    raise ValueError("Expected regular release file: " + name)
                contents[name] = original.extractfile(item).read()
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / "geditor"
            binary.write_bytes(contents["geditor"])
            release.verify_binary(binary, target)
        receipt = {
            "built_as_bottle": True, "poured_from_bottle": False,
            "used_options": [], "unused_options": [], "runtime_dependencies": [],
            "arch": "arm64" if record["arch"] == "arm64" else "x86_64",
            "source": {"tap": repository.split("/")[0] + "/tap", "spec": "stable",
                       "versions": {"stable": current, "head": None, "version_scheme": 0}},
            "changed_files": [], "linkage_files": [], "binary_relocation_files": [],
        }
        output = directory / ("geditor-%s.%s.bottle.tar.gz" % (current, tag))
        with output.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as gz:
            with tarfile.open(fileobj=gz, mode="w") as archive:
                def add(name, data, executable=False):
                    info = tarfile.TarInfo(prefix + "/" + name)
                    info.mode = 0o755 if executable else 0o644
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
                add("bin/geditor", contents.pop("geditor"), True)
                alias = tarfile.TarInfo(prefix + "/bin/ge")
                alias.type = tarfile.SYMTYPE
                alias.linkname = "geditor"
                alias.mode = 0o755
                archive.addfile(alias)
                for name, data in sorted(contents.items()):
                    add(name, data)
                add(".brew/geditor.rb", source_formula.encode())
                add("INSTALL_RECEIPT.json", (json.dumps(receipt, indent=2) + "\n").encode())
        bottles.append((tag, output, release.sha256(output)))
    block = '\n  bottle do\n    root_url "https://github.com/%s/releases/download/v%s"\n' % (repository, current)
    block += "".join('    sha256 cellar: :any_skip_relocation, %s: "%s"\n' % (tag, digest)
                     for tag, _, digest in bottles)
    block += "  end\n"
    formula = source_formula.replace("\n  on_macos do", block + "\n  on_macos do", 1)
    formula_file = directory / "geditor-bottled.rb"
    formula_file.write_text(formula, encoding="utf-8")
    files = [p for _, p, _ in bottles] + [formula_file]
    (directory / "BOTTLE_SHA256SUMS").write_text(
        "".join(release.sha256(p) + "  " + p.name + "\n" for p in sorted(files)), encoding="utf-8")
    return bottles


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    args = parser.parse_args()
    for tag, path, digest in build(args.directory, args.repository):
        print(tag, path.name, digest)
