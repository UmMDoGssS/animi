"""Exercise an extracted Debian package in real WebKitGTK; all data stays temporary.

Usage: python3 scripts/acceptance/linux_packaged.py PACKAGE.deb WebKitWebDriver
Requires a desktop session. No live AniList requests or third-party Python modules.
"""
import base64
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import urllib.request

REPO = Path(__file__).resolve().parents[2]


def wait_for(check):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.1)
    raise AssertionError("Timed out waiting for packaged UI")


def main():
    package, driver_binary = map(lambda arg: str(Path(arg).resolve()), sys.argv[1:3])
    with tempfile.TemporaryDirectory(prefix="animi-packaged-") as temporary:
        root = Path(temporary)
        fixtures = root / "fixtures"
        fixtures.mkdir()
        profile = root / "profile"
        profile.mkdir()
        subprocess.run(["dpkg-deb", "-x", package, str(root / "package")], check=True)
        manga = json.loads((REPO / "src-tauri/tests/fixtures/anilist/manga-search-success.json").read_text())["data"]["Page"]["media"][0]
        manga["coverImage"]["large"] = None
        for kind in ["ANIME", "MANGA"]:
            for operation in ["SearchMedia", "TrendingMedia", "LibraryMedia"]:
                (fixtures / f"{operation}-{kind}.json").write_text(json.dumps({"data": {"Page": {"pageInfo": {"hasNextPage": False}, "media": [manga] if kind == "MANGA" else []}}}))
        (fixtures / "MediaDetails-MANGA.json").write_text(json.dumps({"data": {"Media": dict(manga, relations={"edges": []})}}))
        (fixtures / "AiringSchedule-ANIME.json").write_text('{"data":{"Page":{"airingSchedules":[]}}}')
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        environment = dict(os.environ, TAURI_WEBVIEW_AUTOMATION="true", ANIMI_RELEASE_SMOKE="1", ANIMI_RELEASE_SMOKE_DATA_DIR=str(profile), ANIMI_RELEASE_SMOKE_FIXTURES=str(fixtures))
        driver = subprocess.Popen([driver_binary, f"--port={port}"], env=environment)
        session = None
        base = f"http://127.0.0.1:{port}"

        def request(path, payload=None, method=None):
            data = None if payload is None else json.dumps(payload).encode()
            req = urllib.request.Request(base + path, data=data, headers={"Content-Type": "application/json"}, method=method)
            with urllib.request.urlopen(req, timeout=35) as response:
                value = json.load(response)["value"]
            if isinstance(value, dict) and "error" in value:
                raise AssertionError(value)
            return value

        def start():
            return request("/session", {"capabilities": {"alwaysMatch": {"webkitgtk:browserOptions": {"binary": str(root / "package/usr/bin/animi")}}}})["sessionId"]

        def js(script, *args):
            return request(f"/session/{session}/execute/sync", {"script": script, "args": list(args)})

        def ipc(command, **args):
            value = request(f"/session/{session}/execute/async", {"script": "const done=arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(v=>done({ok:v}),e=>done({error:e}));", "args": [command, args]})
            assert "error" not in value, value
            return value["ok"]

        def click(selector):
            wait_for(lambda: js("return !!document.querySelector(arguments[0])", selector))
            js("document.querySelector(arguments[0]).click()", selector)

        def button(text):
            wait_for(lambda: js("const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===arguments[0]); if(b){b.click();return true} return false", text))

        def text_contains(text):
            return wait_for(lambda: js("return document.body.innerText.includes(arguments[0])", text))

        def fill(selector, value):
            js("const e=document.querySelector(arguments[0]); e.value=arguments[1]; e.dispatchEvent(new Event('input',{bubbles:true})); e.dispatchEvent(new Event('change',{bubbles:true}));", selector, str(value))

        try:
            time.sleep(0.5)
            session = start()
            text_contains("Your library")
            anime = ipc("upsert_library_entry", request={"mediaKind": "ANIME", "anilistId": 101, "status": "PLANNING", "progress": 0, "volumeProgress": None, "score": None, "notes": "Preserve anime"})
            click('button[aria-label="Discover"]')
            click('input[data-media-kind="MANGA"]')
            fill('input[placeholder="Search manga by title"]', "Yotsuba")
            button("Search now")
            click('button[aria-label="Open details for Yotsuba&!"]')
            text_contains("Manga details")
            click('dialog input[value="WATCHING"]')
            fill('dialog input[type="range"]', 42)
            fill('dialog input[type="number"]', 6)
            button("Add to library")
            text_contains("Changes saved")
            fill('dialog input[type="number"]', 5)
            button("Save changes")
            text_contains("Changes saved")
            click('button[aria-label="Close details"]')
            click('button[aria-label="Dashboard"]')
            text_contains("chapters")
            insights = ipc("get_dashboard_insights", request={"mediaKind": "MANGA"})
            assert insights["activity"]["recordedChapters"] == 42, insights
            assert insights["activity"]["recordedVolumes"] == 6, insights
            assert insights["libraryTotals"]["currentVolumeProgress"] == 5
            ipc("refresh_library_metadata", request={"mediaKind": "MANGA"})
            request(f"/session/{session}", method="DELETE")
            session = None
            with sqlite3.connect(profile / "animi.sqlite3") as database:
                database.execute("UPDATE response_caches SET fetched_at = '2000-01-01T00:00:00Z', expires_at = '2000-01-02T00:00:00Z'")
                database.execute("UPDATE media_snapshots SET cached_at = '2000-01-01T00:00:00Z'")
            (fixtures / "offline").touch()
            session = start()
            text_contains("Your library")
            entries = ipc("list_library")
            assert next(i["entry"] for i in entries if i["entry"]["mediaKind"] == "ANIME") == anime
            manga_entry = next(i["entry"] for i in entries if i["entry"]["mediaKind"] == "MANGA")
            assert (manga_entry["progress"], manga_entry["volumeProgress"]) == (42, 5)
            details = ipc("get_media_details", request={"mediaKind": "MANGA", "anilistId": 30013})
            assert details["media"]["chapters"] == 120
            assert details["freshness"]["isStale"] is True
            assert details["freshness"]["refreshError"] is not None
            click('input[data-media-kind="MANGA"]')
            text_contains("Yotsuba&!")
            text_contains("Continue Reading")
            click('button[aria-label="Read one more chapter of Yotsuba&!"]')
            text_contains("chapter 43 recorded")
            assert ipc("get_home_reading")[0]["entry"]["volumeProgress"] == 5
            click('button[aria-label="Dashboard"]')
            text_contains("Your reading history")
            assert ipc("get_dashboard_insights", request={"mediaKind": "MANGA"})["activity"]["recordedChapters"] == 43
            output = Path(os.environ.get("ANIMI_ACCEPTANCE_OUTPUT", "/tmp/animi-acceptance-evidence"))
            output.mkdir(parents=True, exist_ok=True)
            for theme in ["LATTE", "MACCHIATO"]:
                # Theme setting values are persisted through real IPC; reload applies preference.
                ipc("set_theme_preference", preference=theme)
                request(f"/session/{session}/refresh", {})
                text_contains("Your library")
                click('input[data-media-kind="MANGA"]')
                for width in [1180, 390]:
                    request(f"/session/{session}/window/rect", {"width": width, "height": 850})
                    for page in ["Home", "Dashboard", "Discover", "Settings"]:
                        click(f'button[aria-label="{page}"]')
                        time.sleep(0.15)
                        assert js("return document.documentElement.scrollWidth <= window.innerWidth + 1"), (page, width, "overflow")
                        screenshot = request(f"/session/{session}/screenshot")
                        (output / f"{theme.lower()}-{width}-{page.lower()}.png").write_bytes(base64.b64decode(screenshot))
            request(f"/session/{session}", method="DELETE")
            session = start()
            text_contains("Your library")
            assert ipc("get_home_reading")[0]["entry"]["progress"] == 43
            assert ipc("get_dashboard_insights", request={"mediaKind": "MANGA"})["activity"]["recordedChapters"] == 43
            ipc("remove_library_entry", request={"mediaKind": "MANGA", "anilistId": 30013, "confirmed": True})
            assert [i["entry"] for i in ipc("list_library")] == [anime]
            print("PASS: packaged WebKitGTK search, details, add, chapter/volume correction, Dashboard, metadata refresh, restart, offline cache, removal, preserved anime, Continue Reading, durable chapter activity, and theme/viewport screenshots")
        finally:
            if session:
                request(f"/session/{session}", method="DELETE")
            driver.terminate()
            driver.wait(timeout=10)


if __name__ == "__main__":
    main()
