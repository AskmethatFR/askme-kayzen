"""Render the twelve Play Store phone tiles from the raw captures.

`screenshots/` holds the app and nothing else -- 1080x1920, deterministic. A
tile is a poster built from one of those captures: a sand ground, a Fraunces
headline and a Figtree subline in the top 384px (20% of the height), and the
phone held below it in a thin ink frame, ~880px wide, bleeding off the bottom
edge. Headlines stay in ink: cyan means "practised" and nothing else
(docs/functional/design/06-style-galets.md).

This script never captures the app. The tiles are the listing's output, the raw
captures are only their input; `build_screenshots.py` owns the captures.

Each tile has a `crop_top`: the capture row at which the phone's screen starts.
The phone's bottom edge then cuts the capture at `crop_top + VISIBLE_ROWS`. Both
cuts must fall in a gap between two lines of text, found by scanning the
capture row by row, so no line is ever sliced by the bleed.

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
CAPTURE_W = 1080
BAND_MAX = 384
PHONE_W = 880
FRAME = 12
PHONE_TOP = 400
SCREEN_W = PHONE_W - 2 * FRAME
VISIBLE_ROWS = (TILE_H - PHONE_TOP - FRAME) * CAPTURE_W // SCREEN_W
CAPTURE_TIMEOUT = 90
INK_DEVIATION = 60
INK_MIN_PIXELS = 3

SAND = "#F4F1EA"
INK = "#2A2622"
MUTED = "#5E574F"

TILES = {
    "fr": [
        ("01-aujourdhui", 0, "Cinq habitudes au plus.<br>Jamais six.", "Aujourd'hui"),
        ("02-detail", 0, "Sept jours, sans note<br>ni pourcentage.", "Le détail"),
        ("03-rituel", 0, "Un cercle qui respire.<br>Rien d'autre à faire.", "Le rituel"),
        ("04-semaine", 0, "Ce qui a eu lieu.<br>Rien de plus.", "La semaine"),
        ("05-ancrees", 0, "Une habitude ancrée<br>libère une place.", "Ancrées"),
        ("06-ajouter", 0, "Cinq minutes par jour.<br>Un but souple.", "Ajouter"),
    ],
    "en": [
        ("01-aujourdhui", 0, "Five habits at most.<br>Never six.", "Today"),
        ("02-detail", 0, "Seven days, no score<br>and no percentage.", "The detail"),
        ("03-rituel", 0, "A circle that breathes.<br>Nothing else to do.", "The ritual"),
        ("04-semaine", 0, "What happened.<br>Nothing more.", "This week"),
        ("05-ancrees", 0, "An anchored habit<br>frees a seat.", "Anchored"),
        ("06-ajouter", 0, "Five minutes a day.<br>A soft goal.", "Add"),
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
    background: {sand};
    overflow: hidden;
    position: relative;
    text-align: center;
  }}
  .copy {{ position: absolute; top: 76px; left: 48px; right: 48px; }}
  .headline {{
    font-family: 'Fraunces', Georgia, serif;
    font-size: 84px;
    line-height: 1.08;
    color: {ink};
    font-variation-settings: 'wght' 620, 'opsz' 72, 'SOFT' 30;
  }}
  .subline {{
    font-family: 'Figtree', system-ui, sans-serif;
    font-size: 40px;
    font-weight: 500;
    color: {muted};
    margin-top: 20px;
  }}
  .phone {{
    position: absolute;
    top: {phone_top}px; left: {phone_left}px;
    width: {phone_w}px; height: {phone_h}px;
    box-sizing: border-box;
    border: {frame}px solid {ink};
    border-radius: 76px;
    overflow: hidden;
  }}
  .phone img {{
    display: block;
    width: {screen_w}px;
    margin-top: -{crop}px;
  }}
</style>
<div class="copy">
  <div class="headline">{headline}</div>
  <div class="subline">{subline}</div>
</div>
<div class="phone"><img src="{shot}" alt=""></div>
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


def capture_path(locale, name):
    return os.path.join(ROOT, "store", "listing", "screenshots", locale, f"{name}-1080x1920.png")


def ink_rows(img):
    """One bool per row: does a line of content cross it?

    A row is ink when at least a few pixels stand far from the row's median
    luminance -- text on a light screen and on the dark Ritual alike. Card
    edges are too faint to count.
    """
    gray = img.convert("L")
    w, h = gray.size
    px = list(gray.getdata())
    rows = []
    for y in range(h):
        row = px[y * w:(y + 1) * w]
        median = sorted(row)[w // 2]
        rows.append(sum(abs(p - median) > INK_DEVIATION for p in row) >= INK_MIN_PIXELS)
    return rows


def check_text_band(img):
    ink = ink_rows(img.crop((0, 0, img.size[0], PHONE_TOP)))
    last = max((y for y, on in enumerate(ink) if on), default=-1)
    if last + 1 > BAND_MAX:
        raise ValueError(f"text band is {last + 1}px tall, the limit is {BAND_MAX}px")


def check_clean_cuts(ink, crop_top):
    for y in (crop_top, crop_top + VISIBLE_ROWS):
        if 0 < y < len(ink) and (ink[y - 1] or ink[y]):
            raise ValueError(f"the bleed cuts a line of text at capture row {y}")


def compose(chrome, locale, name, crop_top, headline, subline, work):
    from PIL import Image

    shot = capture_path(locale, name)
    check_clean_cuts(ink_rows(Image.open(shot)), crop_top)

    page = TILE_HTML.format(
        title=html_escape.escape(f"{locale} {name}"),
        fonts=FONTS.replace(os.sep, "/"),
        shot=shot.replace(os.sep, "/"),
        w=TILE_W, h=TILE_H, sand=SAND, ink=INK, muted=MUTED,
        phone_top=PHONE_TOP, phone_left=(TILE_W - PHONE_W) // 2,
        phone_w=PHONE_W, phone_h=TILE_H, frame=FRAME, screen_w=SCREEN_W,
        crop=round(crop_top * SCREEN_W / CAPTURE_W),
        headline=headline, subline=html_escape.escape(subline),
    )

    src = os.path.join(work, f"{name}-tile.html")
    raw = os.path.join(work, f"{name}-tile.png")
    open(src, "w", encoding="utf-8").write(page)
    if os.path.exists(raw):
        os.remove(raw)
    run_chrome(
        chrome,
        [
            f"--window-size={TILE_W},{TILE_H}",
            "--force-device-scale-factor=1",
            "--allow-file-access-from-files",
            src,
        ],
        raw,
        f"{locale}/{name} tile",
    )

    tile = Image.open(raw).convert("RGB")
    if tile.size != (TILE_W, TILE_H):
        raise ValueError(f"{locale}/{name}: Chrome wrote {tile.size}, not {TILE_W}x{TILE_H}")
    check_text_band(tile)
    out = os.path.join(OUT, locale, f"{name}-{TILE_W}x{TILE_H}.png")
    tile.save(out, optimize=True)
    return out


def main():
    chrome = store_module().CHROME
    if not os.path.exists(chrome):
        sys.exit(f"headless Chrome not found at {chrome}")

    for locale, tiles in TILES.items():
        work = os.path.join(WORK, locale)
        os.makedirs(work, exist_ok=True)
        os.makedirs(os.path.join(OUT, locale), exist_ok=True)
        for name, crop_top, headline, subline in tiles:
            path = compose(chrome, locale, name, crop_top, headline, subline, work)
            print(f"  {os.path.relpath(path, ROOT)}")


if __name__ == "__main__":
    main()
