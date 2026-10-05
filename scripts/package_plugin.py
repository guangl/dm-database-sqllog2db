"""Package the prebuilt plugin for dameng-cli Release installation (Python 3.7+)."""
import argparse
import re
import tarfile
import zipfile
from pathlib import Path

TARGETS = {
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
}


def package(target, tag, repository=Path("."), dist=Path("dist")):
    if target not in TARGETS:
        raise ValueError("Unsupported release target: " + target)
    # Read only the top-level version fields; fail closed on unexpected layouts.
    versions = []
    for filename in ("Cargo.toml", "dm-plugin.toml"):
        header = (repository / filename).read_text().split("\n[", 1)[0]
        match = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', header, re.MULTILINE)
        if not match:
            raise ValueError("Missing release version in " + filename)
        versions.append(match.group(1))
    if versions[0] != versions[1] or tag != "v" + versions[0]:
        raise ValueError("Release tag, Cargo and plugin versions must match")
    binary = "dm-sqllog2db" + (".exe" if target.endswith("windows-msvc") else "")
    files = [(repository / "dm-plugin.toml", "dm-plugin.toml"),
             (repository / "target" / target / "release" / binary, binary)]
    for source, _ in files:
        if not source.is_file():
            raise ValueError("Missing package input: " + str(source))
    root = "dm-sqllog2db-" + tag + "-" + target
    dist.mkdir(parents=True, exist_ok=True)
    archive = dist / (root + (".zip" if binary.endswith(".exe") else ".tar.gz"))
    if binary.endswith(".exe"):
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as output:
            for source, name in files:
                output.write(source, root + "/" + name)
    else:
        with tarfile.open(archive, "w:gz") as output:
            for source, name in files:
                info = output.gettarinfo(str(source), arcname=root + "/" + name)
                info.mode = 0o755 if name == binary else 0o644
                with source.open("rb") as content:
                    output.addfile(info, content)
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=sorted(TARGETS))
    parser.add_argument("tag")
    args = parser.parse_args()
    package(args.target, args.tag)
