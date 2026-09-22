"""Verify normal Linux startup resolves XDG storage without the release-smoke guard."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

with tempfile.TemporaryDirectory(prefix="animi-xdg-") as directory:
    root = Path(directory)
    environment = {key: value for key, value in os.environ.items() if not key.startswith("ANIMI_RELEASE_SMOKE")}
    environment.update(XDG_DATA_HOME=str(root / "data"), XDG_CONFIG_HOME=str(root / "config"), XDG_CACHE_HOME=str(root / "cache"), TAURI_WEBVIEW_AUTOMATION="true")
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    driver = subprocess.Popen([sys.argv[2], f"--port={port}"], env=environment)
    session = None
    def request(path, payload=None, method=None):
        data = None if payload is None else json.dumps(payload).encode()
        req = urllib.request.Request(f"http://127.0.0.1:{port}" + path, data=data, headers={"Content-Type":"application/json"}, method=method)
        with urllib.request.urlopen(req, timeout=35) as response:
            return json.load(response)["value"]
    try:
        time.sleep(0.5)
        session = request("/session", {"capabilities":{"alwaysMatch":{"webkitgtk:browserOptions":{"binary":str(Path(sys.argv[1]).resolve())}}}})["sessionId"]
        path = root / "data/app.animi.desktop/animi.sqlite3"
        assert path.is_file(), path
        value = request(f"/session/{session}/execute/async", {"script":"const done=arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke('list_library').then(done);", "args":[]})
        assert value == []
        print("PASS: normal packaged Linux startup uses XDG_DATA_HOME/app.animi.desktop without smoke mode")
    finally:
        if session:
            request(f"/session/{session}", method="DELETE")
        driver.terminate()
        driver.wait(timeout=10)
