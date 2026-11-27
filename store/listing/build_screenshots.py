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
of the built index.html before the wasm boots. Dates are computed
relative to FIXED_DAY, and the same copy pins the app's clock and random source
to it, so two runs give identical captures.

The copy also drops the Inter @import that the dx template puts in index.html,
so a capture never depends on reaching fonts.googleapis.com.

Prerequisite: dx build --platform web --package kayzen-app
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
    ("lire-une-page", "Active", [(-24, 3), (-3, 4)],
     [(-10, 3), (-6, 3), (-5, 2), (-3, 4), (-2, 3), (-1, 2), (0, 4)]),
    ("bouger-un-peu", "Active", [(-18, 3)], [(-5, 3), (-4, 2), (-2, 3), (0, 1)]),
    ("respirer-une-minute", "Active", [(-12, 2)],
     [(-6, 2), (-4, 2), (-3, 1), (-1, 2), (0, 2)]),
    ("ecrire-trois-lignes", "Paused", [(-30, 3)], [(-20, 3), (-18, 2), (-15, 3)]),
    ("un-verre-d-eau", "Anchored", [(-70, 3)],
     [(-9, 3), (-8, 3), (-7, 3), (-5, 3), (-4, 3), (-2, 3), (-1, 3), (0, 3)]),
    ("ranger-une-chose", "Anchored", [(-52, 2), (-30, 3)],
     [(-11, 3), (-9, 3), (-8, 3), (-6, 3), (-4, 3), (-3, 3), (-1, 3)]),
]

SHOTS = [
    ("01-aujourdhui", "/"),
    ("02-detail", "/habit/lire-une-page"),
    ("03-rituel", "/habit/respirer-une-minute/ritual"),
    ("04-semaine", "/week"),
    ("05-ancrees", "/anchored"),
    ("06-ajouter", "/add"),
]


# @law: every capture shows FIXED_DAY, so two runs produce the same PNGs.
FIXED_DAY = datetime.date(2026, 3, 17)
BLANK_MAX_COLOURS = 16

# @law: Date and crypto.getRandomValues are shimmed in the staged copy only, never in the dx build.
DATE_SHIM = (
    "(()=>{const R=Date,n=new R(%d,%d,%d,12).getTime();"
    "class D extends R{constructor(...a){a.length?super(...a):super(n)}static now(){return n}}"
    "Date=D;})();"
)
RNG_SHIM = (
    "(()=>{let s=12345;const next=()=>{s=(s+0x6D2B79F5)|0;let t=Math.imul(s^(s>>>15),1|s);"
    "t=(t+Math.imul(t^(t>>>7),61|t))^t;return (t^(t>>>14))&255};"
    "Crypto.prototype.getRandomValues=function(a){"
    "const b=new Uint8Array(a.buffer,a.byteOffset,a.byteLength);"
    "for(let i=0;i<b.length;i++)b[i]=next();return a};})();"
)


def assert_not_blank(path):
    from PIL import Image

    if Image.open(path).convert("RGB").getcolors(maxcolors=BLANK_MAX_COLOURS) is not None:
        raise RuntimeError(f"{path}: blank capture (<= {BLANK_MAX_COLOURS} colours)")


CAPTURE_ATTEMPTS = 3


def publish(shoot, out, attempts=CAPTURE_ATTEMPTS):
    """Shoot into WORK and move into `out` only a capture that has content.

    Chrome sometimes writes a blank PNG when the virtual-time budget runs out
    before the wasm app has rendered (about one run in two on a busy machine).
    A blank shot is retried; the tracked file is never touched until a good
    one exists, so a failed run leaves the previous captures in place.
    """
    from PIL import Image

    raw = os.path.join(WORK, os.path.basename(out))
    for attempt in range(1, attempts + 1):
        shoot(raw)
        Image.open(raw).convert("RGB").resize((OUT_W, OUT_H), Image.LANCZOS).save(raw, optimize=True)
        try:
            assert_not_blank(raw)
        except RuntimeError:
            if attempt == attempts:
                raise
            print(f"  blank capture of {os.path.basename(out)}, retry {attempt}/{attempts - 1}")
            continue
        if attempt > 1:
            print(f"  recovered {os.path.basename(out)} on attempt {attempt}")
        os.replace(raw, out)
        return out


def today_as_stored():
    """The integer a stored date carries.

    `SystemClock` feeds `LocalDate::from_epoch_day()` the value of chrono's
    `num_days_from_ce()`, so a stored day is a proleptic-Gregorian ordinal
    (~739_000), not a count of days since 1970 -- `date.toordinal()` is its
    exact Python equivalent. Seeding epoch days instead silently produces a
    board whose completions all sit ~719_000 days in the past.
    """
    return FIXED_DAY.toordinal()


def snapshot(locale):
    today = today_as_stored()
    titles = TITLES[locale]
    return {
        "v": 2,
        "habits": [
            {
                "id": hid,
                "title": titles[hid],
                "state": state,
                "steps": [{"on": today + d, "goal": g} for d, g in steps],
                "completions": [today + d for d, n in done for _ in range(n)],
            }
            for hid, state, steps, done in BOARD
        ],
    }


def stage(locale, viewport_h):
    """Copy the build, seed the board, drop the remote fonts, pin the viewport.

    The zoom is what buys a phone layout: Chrome refuses a window narrower
    than 500 CSS pixels, so shrinking the CSS pixel by 500/360 is the only way
    left to hand the page the 360dp viewport the Android app actually runs in.

    `.screen.ritual` and `.screen.anchored` are `min-height: 100dvh`, and under
    that zoom the unit is the window scaled up again: the screen came out
    taller than the frame, which pushed the ritual's completion button and stop
    link below the fold. They are pinned to the phone's `viewport_h` instead.
    The `html` prefix outranks main.css, which dx links after this style.
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
    shims = DATE_SHIM % (FIXED_DAY.year, FIXED_DAY.month - 1, FIXED_DAY.day) + RNG_SHIM
    seed = (
        f"<script>{shims}"
        f"localStorage.setItem({json.dumps(STORAGE_KEY)}, {payload});</script>"
    )
    html = html.replace("<head>", "<head>\n" + seed, 1)
    pin = f"html .screen.ritual,html .screen.anchored{{min-height:{viewport_h}px}}"
    still = "*{transition:none!important}"
    html = html.replace(
        "</head>", f"<style>html{{zoom:{ZOOM:.6f}}}{pin}{still}</style>\n</head>", 1
    )
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
    out = os.path.join(out_dir, f"{name}-{OUT_W}x{OUT_H}.png")
    return publish(lambda raw: chrome_shot(locale, name, route, raw), out)


def chrome_shot(locale, name, route, out):
    """Shoot one route into `out`, then kill the browser.

    Chrome writes the PNG when its virtual-time budget expires but does not
    reliably exit afterwards on a page that keeps animating -- Kayzen's ritual
    circle breathes on an infinite loop. Waiting on the process therefore hangs
    on a capture that already succeeded, so the file itself is the completion
    signal: it is polled until its size stops growing, and the browser is then
    terminated.

    The screen's entry fade (`kzUp`) does not advance under the virtual clock,
    so without `--force-prefers-reduced-motion` the PNG catches a near-invisible
    screen next to a crisp nav.
    """
    os.makedirs(WORK, exist_ok=True)
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
            "--force-prefers-reduced-motion",
            "--virtual-time-budget=6000",
            "--run-all-compositor-stages-before-draw",
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


def main():
    if not os.path.isdir(PUBLIC):
        sys.exit("no web build found -- run: dx build --platform web --package kayzen-app")
    if not os.path.exists(CHROME):
        sys.exit(f"headless Chrome not found at {CHROME}")

    from PIL import Image

    for locale in ("fr", "en"):
        root = stage(locale, VIEWPORT_H)
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
