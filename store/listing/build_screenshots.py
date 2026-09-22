"""Capture the Play Store phone screenshots from the real app.

The Android app is a Dioxus WebView, so the web build renders the same DOM and
the same stylesheet as the shipped binary -- these are captures of the app, not
of the prototype mock-ups in docs/functional/design/images/.

The viewport is a phone, not a 1080px-wide desktop: the layout must be the one
a 360dp Android screen gets, or the captures show a tablet column.

Reaching 360dp takes a detour. `--window-size` IS the CSS viewport and the file
Chrome writes is that window multiplied by the scale factor -- there is no
division anywhere -- but Chrome clamps the window to a floor of 500 CSS pixels,
in old and new headless alike. Asking for 360 lays the page out at 500 and then
photographs the leftmost 360 of it: content silently cut off at the right edge.
So the window is asked for at that 500-pixel floor and `html { zoom }` shrinks
a CSS pixel by the same ratio, which puts the layout back at 360dp with nothing
clipped. The capture lands above Play's 1080x1920 and is resampled down to it.

The demo board is seeded straight into `localStorage` under the key the web
composition root uses (`kayzen.habits.v1`), by injecting one script into a copy
of the built index.html before the wasm boots. Dates are computed relative to
the day the script runs, so the seven-day staircase always lands inside its
window.

The copy also drops the Inter @import that the dx template puts in index.html,
so a capture never depends on reaching fonts.googleapis.com.

Prerequisite: dx build --platform web
Usage: python3 store/listing/build_screenshots.py
"""

import datetime
import functools
import http.server
import json
import os
import shutil
import socketserver
import subprocess
import sys
import threading
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PUBLIC = os.path.join(ROOT, "target", "dx", "kayzen-app", "debug", "web", "public")
OUT = os.path.join(ROOT, "store", "listing", "screenshots")
WORK = os.path.join(ROOT, "target", "store-screenshots")
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
PORT = 8731
PHONE_CSS_WIDTH = 360
CHROME_MIN_WINDOW_W = 500
ZOOM = CHROME_MIN_WINDOW_W / PHONE_CSS_WIDTH
VIEWPORT_W, VIEWPORT_H = PHONE_CSS_WIDTH, 640
CAPTURE_DPR = 3
WINDOW_W, WINDOW_H = CHROME_MIN_WINDOW_W, round(VIEWPORT_H * ZOOM)
OUT_W, OUT_H = 1080, 1920
CAPTURE_TIMEOUT = 90
STORAGE_KEY = "kayzen.habits.v1"

TITLES = {
    "fr": {
        "lire-une-page": "Lire une page",
        "bouger-un-peu": "Bouger un peu",
        "respirer-une-minute": "Respirer une minute",
        "ecrire-trois-lignes": "Écrire trois lignes",
        "un-verre-d-eau": "Un verre d'eau au réveil",
        "ranger-une-chose": "Ranger une chose",
    },
    "en": {
        "lire-une-page": "Read one page",
        "bouger-un-peu": "Move a little",
        "respirer-une-minute": "Breathe for a minute",
        "ecrire-trois-lignes": "Write three lines",
        "un-verre-d-eau": "A glass of water on waking",
        "ranger-une-chose": "Put one thing away",
    },
}

# @algo: a habit's bar stands at the goal that was active on its own day, so a
# step dated before a completion is what makes the staircase rise -- the demo
# board grows "Lire une page" twice so the seven-day window shows two heights.
BOARD = [
    ("lire-une-page", "Active", [(-24, 5), (-9, 6), (-3, 7)], [-6, -5, -3, -1, 0]),
    ("bouger-un-peu", "Active", [(-18, 5)], [-5, -4, -2]),
    ("respirer-une-minute", "Active", [(-12, 3)], [-6, -4, -3, -1, 0]),
    ("ecrire-trois-lignes", "Paused", [(-30, 5)], [-20, -18, -15]),
    ("un-verre-d-eau", "Anchored", [(-70, 5)], [-9, -8, -7, -5, -4, -2, -1, 0]),
    ("ranger-une-chose", "Anchored", [(-52, 5), (-30, 6)], [-11, -9, -8, -6, -4, -3, -1]),
]

SHOTS = [
    ("01-aujourdhui", "/"),
    ("02-detail", "/habit/lire-une-page"),
    ("03-rituel", "/habit/respirer-une-minute/ritual"),
    ("04-semaine", "/week"),
    ("05-ancrees", "/anchored"),
    ("06-ajouter", "/add"),
]


def today_as_stored():
    """The integer a stored date carries.

    `SystemClock` feeds `LocalDate::from_epoch_day()` the value of chrono's
    `num_days_from_ce()`, so a stored day is a proleptic-Gregorian ordinal
    (~739_000), not a count of days since 1970 -- `date.toordinal()` is its
    exact Python equivalent. Seeding epoch days instead silently produces a
    board whose completions all sit ~719_000 days in the past.
    """
    return datetime.date.today().toordinal()


def snapshot(locale):
    today = today_as_stored()
    titles = TITLES[locale]
    return {
        "v": 1,
        "habits": [
            {
                "id": hid,
                "title": titles[hid],
                "state": state,
                "steps": [{"on": today + d, "goal": g} for d, g in steps],
                "completions": [today + d for d in done],
            }
            for hid, state, steps, done in BOARD
        ],
    }


def stage(locale):
    """Copy the build, seed the board, drop the remote fonts, pin the viewport.

    The zoom is what buys a phone layout: Chrome refuses a window narrower
    than 500 CSS pixels, so shrinking the CSS pixel by 500/360 is the only way
    left to hand the page the 360dp viewport the Android app actually runs in.
    """
    root = os.path.join(WORK, locale)
    if os.path.exists(root):
        shutil.rmtree(root)
    shutil.copytree(PUBLIC, root)

    index = os.path.join(root, "index.html")
    html = open(index, encoding="utf-8").read()
    before = html.count("fonts.googleapis.com")
    html = "\n".join(
        line for line in html.splitlines() if "fonts.googleapis.com" not in line
    )
    payload = json.dumps(json.dumps(snapshot(locale), ensure_ascii=False))
    seed = f"<script>localStorage.setItem({json.dumps(STORAGE_KEY)}, {payload});</script>"
    html = html.replace("<head>", "<head>\n" + seed, 1)
    html = html.replace("</head>", f"<style>html{{zoom:{ZOOM:.6f}}}</style>\n</head>", 1)
    open(index, "w", encoding="utf-8").write(html)
    print(f"  staged {locale}: seeded board, dropped {before} remote font import(s)")
    return root


class Spa(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        full = super().translate_path(path)
        if os.path.isdir(full) or os.path.exists(full):
            return full
        return os.path.join(self.directory, "index.html")

    def log_message(self, *args):
        pass


def serve(directory):
    """Threaded on purpose.

    Chrome opens several connections at once for the wasm bundle and the
    stylesheets; a single-threaded server serialises them behind one keep-alive
    socket and the page never finishes loading, which reads as a hung capture
    rather than as a server problem.
    """
    handler = functools.partial(Spa, directory=directory)
    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def capture(locale, name, route, out_dir):
    """Shoot one route, then kill the browser.

    Chrome writes the PNG when its virtual-time budget expires but does not
    reliably exit afterwards on a page that keeps animating -- Kayzen's ritual
    circle breathes on an infinite loop. Waiting on the process therefore hangs
    on a capture that already succeeded, so the file itself is the completion
    signal: it is polled until its size stops growing, and the browser is then
    terminated.
    """
    from PIL import Image

    out = os.path.join(out_dir, f"{name}-{OUT_W}x{OUT_H}.png")
    if os.path.exists(out):
        os.remove(out)

    proc = subprocess.Popen(
        [
            CHROME, "--headless", "--disable-gpu",
            "--no-first-run", "--no-default-browser-check",
            "--disable-background-networking", "--disable-sync",
            "--disable-default-apps", "--disable-extensions",
            f"--screenshot={out}",
            f"--window-size={WINDOW_W},{WINDOW_H}",
            f"--force-device-scale-factor={CAPTURE_DPR}",
            "--hide-scrollbars",
            "--virtual-time-budget=6000",
            f"--lang={locale}",
            f"--accept-lang={'fr-FR,fr' if locale == 'fr' else 'en-US,en'}",
            f"--user-data-dir={os.path.join(WORK, '.chrome-' + locale)}",
            f"http://127.0.0.1:{PORT}{route}",
        ],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )

    settled, deadline = -1, time.time() + CAPTURE_TIMEOUT
    try:
        while time.time() < deadline:
            time.sleep(0.5)
            size = os.path.getsize(out) if os.path.exists(out) else -1
            if size > 0 and size == settled:
                break
            settled = size
        else:
            raise TimeoutError(f"{locale}/{name}: no screenshot after {CAPTURE_TIMEOUT}s")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()

    Image.open(out).convert("RGB").resize((OUT_W, OUT_H), Image.LANCZOS).save(out, optimize=True)
    return out


def main():
    if not os.path.isdir(PUBLIC):
        sys.exit("no web build found -- run: dx build --platform web")
    if not os.path.exists(CHROME):
        sys.exit(f"headless Chrome not found at {CHROME}")

    from PIL import Image

    for locale in ("fr", "en"):
        root = stage(locale)
        httpd = serve(root)
        out_dir = os.path.join(OUT, locale)
        os.makedirs(out_dir, exist_ok=True)
        try:
            for name, route in SHOTS:
                path = capture(locale, name, route, out_dir)
                size = Image.open(path).size
                print(f"  {os.path.relpath(path, ROOT)}  {size[0]}x{size[1]}")
        finally:
            httpd.shutdown()
            httpd.server_close()


if __name__ == "__main__":
    main()
