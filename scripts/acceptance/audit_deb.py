"""Audit the private Linux payload without installing or executing it."""
import hashlib
import io
from pathlib import Path
import subprocess
import sys
import tarfile

package = Path(sys.argv[1])
fields = subprocess.check_output(["dpkg-deb", "-f", str(package), "Package", "Version", "Architecture"], text=True)
assert "Package: animi" in fields and "Version: 1.1.0" in fields and "Architecture: amd64" in fields, fields
payload = subprocess.check_output(["dpkg-deb", "--fsys-tarfile", str(package)])
required = {
    "usr/bin/animi",
    "usr/share/applications/Animi.desktop",
    "usr/share/doc/animi/copyright",
    "usr/share/doc/animi/THIRD_PARTY_NOTICES.md",
}
with tarfile.open(fileobj=io.BytesIO(payload)) as archive:
    files = {member.name.removeprefix("./") for member in archive if member.isfile()}
    assert required <= files, required - files
    for member in archive:
        assert member.isdir() or member.isfile(), member.name
    for path in files:
        assert path in required or (path.startswith("usr/share/icons/hicolor/") and path.endswith("/apps/animi.png")), path
    desktop = archive.extractfile("usr/share/applications/Animi.desktop").read().decode()
    assert "Exec=animi\n" in desktop and "Terminal=false" in desktop and "Categories=Utility;" in desktop
    assert "http" not in desktop
controls = subprocess.check_output(["dpkg-deb", "--ctrl-tarfile", str(package)])
with tarfile.open(fileobj=io.BytesIO(controls)) as archive:
    assert {member.name.removeprefix("./") for member in archive if member.isfile()} <= {"control", "md5sums"}, "Unexpected maintainer scripts"
print(f"PASS: private Debian payload allowlist, native desktop integration, notices, no maintainer scripts or bundled data; SHA256 {hashlib.sha256(package.read_bytes()).hexdigest()}")
