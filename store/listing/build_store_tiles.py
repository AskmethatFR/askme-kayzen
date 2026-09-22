"""Render the six Play Store phone tiles: a framed app screen under a headline.

The raw captures in `screenshots/` are the app and nothing else -- honest, but
a bare 1080x1920 screen sells nothing at the size Play actually renders it. A
store tile is a poster: a background, one line of copy, and the app held inside
a device. The shape is borrowed from Goldie (github.com/kacperkapusciak/goldie)
-- its `classic` template: headline above, device whole below -- and dressed
entirely in Galets, so the tile and the app it advertises share one palette and
one pair of typefaces.

Goldie itself drives a real emulator against a built APK and wants a Node
toolchain; Kayzen already owns a headless-Chrome capture pipeline, so only the
composition is rebuilt here. The demo board, the staging and the server come
from `build_screenshots.py` -- imported, never copied, so the tiles can never
disagree with the raw captures about what the board contains.

Each tile's kicker names the screen and its headline states the one promise
that screen keeps. Both obey the listing's writing rules (see `copy.fr.md`):
no superlative, no ranking, no promised outcome -- an app that refuses to
measure progress cannot advertise it.

The device holds a real 360dp phone layout, obtained the way
`build_screenshots.py` documents: Chrome clamps its window to 500 CSS pixels,
so the window sits at that floor and the staged page's `html { zoom }` shrinks
the CSS pixel back to a phone's. The tile is composed at twice its final size
and resampled down once, and the capture is always larger than the slot it
fills, so no pixel is ever invented.

Prerequisite: dx build --platform web
Usage: python3 store/listing/build_store_tiles.py
"""

import html as html_escape
import importlib.util
import os
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "store", "listing", "tiles")
WORK = os.path.join(ROOT, "target", "store-tiles")
FONTS = os.path.join(ROOT, "app", "assets", "fonts")

TILE_W, TILE_H = 1080, 1920
SCALE = 2
VIEWPORT_W, VIEWPORT_H = 360, 840
SCREEN_W = 660
SCREEN_H = round(SCREEN_W * VIEWPORT_H / VIEWPORT_W)
CAPTURE_DPR = 3
BEZEL = 14
DEVICE_TOP = 292
CAPTURE_TIMEOUT = 90

PAPER = {
    "bg": "#F4F1EA",
    "blob": "#DDEEF3",
    "kicker": "#00708F",
    "ink": "#2A2622",
    "bezel": "#2A2622",
    "shadow": "rgba(42, 38, 34, 0.20)",
}
OCRE = {**PAPER, "blob": "#EFE3C8", "kicker": "#7A5E24"}

TILES = {
    "fr": [
        ("01-aujourdhui", PAPER, "Aujourd'hui", "Cinq habitudes au plus.<br>Jamais six."),
        ("02-detail", PAPER, "Le détail", "Sept jours, sans note<br>ni pourcentage."),
        ("03-rituel", PAPER, "Le rituel", "Un cercle qui respire.<br>Rien d'autre à faire."),
        ("04-semaine", PAPER, "La semaine", "Ce qui a eu lieu.<br>Rien de plus."),
        ("05-ancrees", OCRE, "Ancrées", "Une habitude ancrée<br>libère une place."),
        ("06-ajouter", PAPER, "Ajouter", "Cinq minutes par jour.<br>Un but souple."),
    ],
    "en": [
        ("01-aujourdhui", PAPER, "Today", "Five habits at most.<br>Never six."),
        ("02-detail", PAPER, "The detail", "Seven days, no score<br>and no percentage."),
        ("03-rituel", PAPER, "The ritual", "A circle that breathes.<br>Nothing else to do."),
        ("04-semaine", PAPER, "This week", "What happened.<br>Nothing more."),
        ("05-ancrees", OCRE, "Anchored", "An anchored habit<br>frees a seat."),
        ("06-ajouter", PAPER, "Add", "Five minutes a day.<br>A soft goal."),
    ],
}

TILE_HTML = """<!doctype html>
<meta charset="utf-8">
<title>{title}</title>
<style>
  @font-face {{
    font-family: 'Fraunces';
    src: url('{fonts}/Fraunces-Variable.woff2') format('woff2');
    font-weight: 100 900;
  }}
  @font-face {{
    font-family: 'Figtree';
    src: url('{fonts}/Figtree-Variable.woff2') format('woff2');
    font-weight: 300 900;
  }}
  html, body {{ margin: 0; padding: 0; }}
  body {{
    width: {w}px; height: {h}px;
    background: {bg};
    overflow: hidden;
    position: relative;
  }}
  .blob {{
    position: absolute;
    top: {blob_top}px; right: {blob_right}px;
    width: {blob}px; height: {blob}px;
    border-radius: 48% 52% 46% 54% / 55% 47% 53% 45%;
    background: {blob_color};
  }}
  .copy {{
    position: absolute;
    top: {pad}px; left: {pad}px; right: {pad}px;
  }}
  .kicker {{
    font-family: 'Figtree', system-ui, sans-serif;
    font-size: {kicker}px;
    font-weight: 600;
    letter-spacing: {track}px;
    text-transform: uppercase;
    color: {kicker_color};
    margin-bottom: {gap}px;
  }}
  .headline {{
    font-family: 'Fraunces', Georgia, serif;
    font-size: {headline}px;
    line-height: 1.16;
    color: {ink};
    font-variation-settings: 'wght' 620, 'opsz' 72, 'SOFT' 30;
  }}
  .device {{
    position: absolute;
    top: {device_top}px; left: {device_left}px;
    width: {device_w}px; height: {device_h}px;
    box-sizing: border-box;
    padding: {bezel}px;
    border-radius: {device_radius}px;
    background: {bezel_color};
    box-shadow: 0 {shadow_y}px {shadow_blur}px {shadow_color};
  }}
  .device img {{
    display: block;
    width: {screen_w}px; height: {screen_h}px;
    border-radius: {screen_radius}px;
  }}
</style>
<div class="blob"></div>
<div class="copy">
  <div class="kicker">{kicker_text}</div>
  <div class="headline">{headline_text}</div>
</div>
<div class="device"><img src="{shot}" alt=""></div>
"""


def store_module():
    path = os.path.join(ROOT, "store", "listing", "build_screenshots.py")
    spec = importlib.util.spec_from_file_location("kayzen_store_captures", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_chrome(chrome, args, out, label):
    """Drive one headless Chrome shot, then kill the browser.

    Chrome writes the PNG when its virtual-time budget expires but does not
    reliably exit on a page that keeps animating -- the ritual circle breathes
    forever. The file settling at a stable size is the completion signal.
    """
    proc = subprocess.Popen(
        [
            chrome, "--headless", "--disable-gpu",
            "--no-first-run", "--no-default-browser-check",
            "--disable-background-networking", "--disable-sync",
            "--disable-default-apps", "--disable-extensions",
            "--hide-scrollbars", "--virtual-time-budget=6000",
            f"--screenshot={out}",
            *args,
        ],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )

    settled, deadline = -1, time.time() + CAPTURE_TIMEOUT
    try:
        while time.time() < deadline:
            time.sleep(0.5)
            size = os.path.getsize(out) if os.path.exists(out) else -1
            if size > 0 and size == settled:
                return
            settled = size
        raise TimeoutError(f"{label}: nothing written after {CAPTURE_TIMEOUT}s")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


def capture_screen(bs, locale, name, route, work):
    out = os.path.join(work, f"{name}-screen.png")
    if os.path.exists(out):
        os.remove(out)
    run_chrome(
        bs.CHROME,
        [
            f"--window-size={bs.CHROME_MIN_WINDOW_W},{round(VIEWPORT_H * bs.ZOOM)}",
            f"--force-device-scale-factor={CAPTURE_DPR}",
            f"--lang={locale}",
            f"--accept-lang={'fr-FR,fr' if locale == 'fr' else 'en-US,en'}",
            f"--user-data-dir={os.path.join(work, '.chrome')}",
            f"http://127.0.0.1:{bs.PORT}{route}",
        ],
        out,
        f"{locale}/{name} screen",
    )
    return out


def compose(chrome, name, theme, kicker, headline, shot, work, out_dir):
    from PIL import Image

    s = SCALE
    device_w = SCREEN_W + 2 * BEZEL
    device_h = SCREEN_H + 2 * BEZEL
    page = TILE_HTML.format(
        title=html_escape.escape(f"{kicker} - {name}"),
        fonts=FONTS.replace(os.sep, "/"),
        shot=shot.replace(os.sep, "/"),
        w=TILE_W * s, h=TILE_H * s,
        bg=theme["bg"], ink=theme["ink"],
        blob_color=theme["blob"], kicker_color=theme["kicker"],
        bezel_color=theme["bezel"], shadow_color=theme["shadow"],
        blob=420 * s, blob_top=-130 * s, blob_right=-110 * s,
        pad=64 * s, kicker=24 * s, track=int(3 * s), gap=18 * s,
        headline=52 * s,
        device_top=DEVICE_TOP * s,
        device_left=((TILE_W - device_w) // 2) * s,
        device_w=device_w * s, device_h=device_h * s,
        bezel=BEZEL * s, device_radius=58 * s, screen_radius=(58 - BEZEL) * s,
        screen_w=SCREEN_W * s, screen_h=SCREEN_H * s,
        shadow_y=28 * s, shadow_blur=70 * s,
        kicker_text=html_escape.escape(kicker), headline_text=headline,
    )

    src = os.path.join(work, f"{name}-tile.html")
    raw = os.path.join(work, f"{name}-tile.png")
    open(src, "w", encoding="utf-8").write(page)
    if os.path.exists(raw):
        os.remove(raw)
    run_chrome(
        chrome,
        [
            f"--window-size={TILE_W * s},{TILE_H * s}",
            "--force-device-scale-factor=1",
            "--allow-file-access-from-files",
            src,
        ],
        raw,
        f"{name} tile",
    )

    out = os.path.join(out_dir, f"{name}-{TILE_W}x{TILE_H}.png")
    Image.open(raw).convert("RGB").resize((TILE_W, TILE_H), Image.LANCZOS).save(
        out, optimize=True
    )
    return out


def main():
    bs = store_module()
    if not os.path.isdir(bs.PUBLIC):
        sys.exit("no web build found -- run: dx build --platform web")
    if not os.path.exists(bs.CHROME):
        sys.exit(f"headless Chrome not found at {bs.CHROME}")

    from PIL import Image

    routes = dict(bs.SHOTS)
    for locale, tiles in TILES.items():
        work = os.path.join(WORK, locale)
        os.makedirs(work, exist_ok=True)
        out_dir = os.path.join(OUT, locale)
        os.makedirs(out_dir, exist_ok=True)

        root = bs.stage(locale)
        httpd = bs.serve(root)
        try:
            for name, theme, kicker, headline in tiles:
                shot = capture_screen(bs, locale, name, routes[name], work)
                path = compose(bs.CHROME, name, theme, kicker, headline, shot, work, out_dir)
                size = Image.open(path).size
                print(f"  {os.path.relpath(path, ROOT)}  {size[0]}x{size[1]}")
        finally:
            httpd.shutdown()
            httpd.server_close()


if __name__ == "__main__":
    main()
