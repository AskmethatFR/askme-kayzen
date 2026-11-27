"""Render the twelve Play Store phone tiles from the raw captures.

`screenshots/` holds the app and nothing else -- 1080x1920, deterministic. A
tile is a poster built from one of those captures: a sand-to-pale-cyan wash with
one blurred pebble behind a 940px Android phone (thin ink bezel, status bar,
punch-hole, soft shadow), a Fraunces headline and a Figtree subline in a 300px
band, and one zoomed callout cropped from the same capture. Headlines stay in
ink: cyan means "practised" and nothing else
(docs/functional/design/06-style-galets.md). Colours are the Galets tokens.

Each tile has a `layout`: "top" puts the caption above the phone, which bleeds
off the bottom edge and shows its status bar; "bottom" puts the caption, centred
in its band, under the phone, which is cropped by the top edge. `crop` is the
callout's rectangle in capture pixels and `zoom` its enlargement over the
phone's own scale. The callout is a lens: it is centred on the source rectangle
it was cropped from and so hides it (clamped to stay on the tile, clear of the
caption band). Headlines are measured in Chrome: at most two lines and never a
lone word on the last one.

This script never captures the app. The tiles are the listing's output, the raw
captures are only their input; `build_screenshots.py` owns the captures.

Each tile has a `crop_top`: the capture row at the tile's phone-screen edge that
bleeds. The phone's other edge then cuts the capture at `crop_top +
VISIBLE_ROWS`. Both cuts must fall in a gap between two lines of text, found by
scanning the capture row by row, so no line is ever sliced by the bleed.

Usage: python3 store/listing/build_store_tiles.py
"""

import html as html_escape
import importlib.util
import json
import os
import re
import subprocess
import sys
import time
from typing import NamedTuple

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "store", "listing", "tiles")
WORK = os.path.join(ROOT, "target", "store-tiles")
FONTS = os.path.join(ROOT, "app", "assets", "fonts")

TILE_W, TILE_H = 1080, 1920
CAPTURE_W = 1080
BAND_MAX = 284
PHONE_W = 940
PHONE_LEFT = (TILE_W - PHONE_W) // 2
FRAME = 10
PHONE_TOP = 300
STATUS_BAR_H = 56
SCREEN_RADIUS = 58
SCREEN_W = PHONE_W - 2 * FRAME
SCALE = SCREEN_W / CAPTURE_W
CALLOUT_BORDER = 6
SHADOW_REACH = 80
CALLOUT_EDGE = 8
COPY_SIDE = 48
VISIBLE_ROWS = (TILE_H - PHONE_TOP - FRAME) * CAPTURE_W // SCREEN_W
CAPTURE_TIMEOUT = 90
INK_DEVIATION = 100
INK_MIN_PIXELS = 3

SAND = "#F4F1EA"
INK = "#2A2622"
MUTED = "#5E574F"

class Tile(NamedTuple):
    name: str
    capture: str
    crop_top: int
    headline: str
    subline: str
    layout: str
    crop: tuple
    zoom: float


TILES = {
    "fr": [
        Tile("01-aujourdhui", "01-aujourdhui", 0, "Un petit pas suffit.", "Pas de série, pas de pression.",
             "top", (125, 1075, 985, 1303), 1.4),
        Tile("02-detail", "02-detail", 30, "Un jour sans ? Rien ne casse.", "Aucune série à protéger.",
             "bottom", (102, 1360, 972, 1512), 1.4),
        Tile("03-semaine", "04-semaine", 0, "Votre rythme, pas le nôtre.", "Plus ou moins : vous décidez.",
             "top", (112, 1190, 968, 1392), 1.4),
        Tile("04-rituel", "03-rituel", 0, "Un moment pour vous.", "Arrêter n'est pas échouer.",
             "top", (100, 1440, 980, 1740), 1.4),
        Tile("05-ancrees", "05-ancrees", 30, "Devenue naturelle ? Ancrez-la.", "Elle libère une place.",
             "bottom", (100, 560, 880, 880), 1.4),
        Tile("06-ajouter", "06-ajouter", 0, "Commencez tout petit.", "Cinq habitudes au plus.",
             "top", (110, 1235, 975, 1555), 1.4),
    ],
    "en": [
        Tile("01-aujourdhui", "01-aujourdhui", 0, "One small step is enough.", "No streaks, no pressure.",
             "top", (125, 1075, 985, 1303), 1.4),
        Tile("02-detail", "02-detail", 30, "Skip a day. Nothing breaks.", "No streak to protect.",
             "bottom", (102, 1360, 972, 1512), 1.4),
        Tile("03-semaine", "04-semaine", 0, "Your pace, not ours.", "More or less: you decide.",
             "top", (112, 1280, 968, 1480), 1.4),
        Tile("04-rituel", "03-rituel", 0, "A moment for yourself.", "Stopping isn't failing.",
             "top", (100, 1440, 980, 1740), 1.4),
        Tile("05-ancrees", "05-ancrees", 30, "Became natural? Anchor it.", "It frees up a seat.",
             "bottom", (100, 560, 960, 880), 1.4),
        Tile("06-ajouter", "06-ajouter", 0, "Start tiny.", "Five habits at most.",
             "top", (110, 1235, 975, 1555), 1.4),
    ],
}

HEADLINE_MAX_LINES = 2
HEADLINE_MAX_WORDS = 7
SUBLINE_MAX_WORDS = 6

HEADLINE_CSS = """.headline {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 64px;
    line-height: 1.1;
    color: #2A2622;
    font-variation-settings: 'wght' 620, 'opsz' 72, 'SOFT' 30;
    text-wrap: balance;
  }"""

MEASURE_HTML = """<!doctype html>
<meta charset="utf-8">
<style>
  @font-face {{
    font-family: 'Fraunces';
    src: url('{fonts}/Fraunces-Variable.woff2') format('woff2');
    font-weight: 100 900;
  }}
  html, body {{ margin: 0; padding: 0; }}
  body {{ width: {w}px; }}
  .copy {{ margin: 0 {side}px; text-align: center; }}
  {headline_css}
</style>
<div class="copy">{headlines}</div>
<pre id="out"></pre>
<script>
  document.fonts.load("620 96px Fraunces").then(() => {{
    const result = [...document.querySelectorAll(".headline")].map(line => {{
      const tops = [...line.querySelectorAll("span")].map(s => Math.round(s.getBoundingClientRect().top));
      const last = Math.max(...tops);
      return [new Set(tops).size, tops.filter(t => t === last).length];
    }});
    document.getElementById("out").textContent = JSON.stringify(result);
  }});
</script>
"""

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
    background: linear-gradient({wash_angle}, {sand} 0%, {tint} 36%, {tint} 100%);
    overflow: hidden;
    position: relative;
    text-align: center;
  }}
  .galet {{
    position: absolute; {galet_at}
    width: 1000px; height: 940px;
    border-radius: {pebble};
    background: {accent};
    opacity: .2;
    filter: blur(90px);
  }}
  .copy {{
    position: absolute; {copy_edge}: 0; left: {side}px; right: {side}px; height: {band}px;
    box-sizing: border-box; padding: {copy_pad};
    display: flex; flex-direction: column; justify-content: {copy_justify};
  }}
  {headline_css}
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
    border-radius: {phone_radius}px;
    background: {ink};
    overflow: hidden;
    box-shadow: 0 44px 72px -12px rgba({shadow}, .30), 0 12px 26px rgba({shadow}, .16);
  }}
  .phone img {{
    display: block;
    width: {screen_w}px;
    margin-top: -{crop}px;
  }}
  .statusbar {{
    position: absolute; top: 0; left: 0; right: 0; height: {status_h}px;
    box-sizing: border-box; padding: 4px 46px 0 50px;
    display: flex; align-items: center; justify-content: space-between;
    color: {status_ink};
    font: 600 26px 'Figtree', system-ui, sans-serif;
  }}
  .statusbar .glyphs {{ display: flex; align-items: center; gap: 12px; }}
  .hole {{
    position: absolute; top: 17px; left: 50%; width: 24px; height: 24px; margin-left: -12px;
    border-radius: 50%; background: {ink};
  }}
  .callout {{
    position: absolute; left: {callout_x}px; top: {callout_y}px;
    width: {callout_w}px; height: {callout_h}px;
    border: {callout_border}px solid {card};
    border-radius: 30px;
    background: url('{shot}') no-repeat {callout_bg_x}px {callout_bg_y}px / {callout_bg_w}px auto;
    box-shadow: 0 26px 48px rgba({shadow}, .24), 0 6px 14px rgba({shadow}, .14);
  }}
</style>
<div class="galet"></div>
<div class="copy">
  <div class="headline">{headline}</div>
  <div class="subline">{subline}</div>
</div>
<div class="phone"><img src="{shot}" alt="">{statusbar}</div>
<div class="callout"></div>
"""

STATUSBAR_HTML = """
  <div class="statusbar"><span>9:41</span><span class="glyphs">
    <svg width="30" height="22" viewBox="0 0 30 22" fill="currentColor"><path d="M2 20h4v-4H2zm7 0h4v-8H9zm7 0h4V7h-4zm7 0h4V2h-4z"/></svg>
    <svg width="28" height="22" viewBox="0 0 28 22" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round"><path d="M2 8a17 17 0 0 1 24 0M6.5 12.5a10.5 10.5 0 0 1 15 0M11 17a4 4 0 0 1 6 0"/></svg>
    <svg width="42" height="22" viewBox="0 0 42 22"><rect x="1" y="3" width="34" height="16" rx="5" fill="none" stroke="currentColor" stroke-width="2" opacity=".55"/><rect x="4.5" y="6.5" width="23" height="9" rx="2.5" fill="currentColor"/><rect x="37" y="8" width="3" height="6" rx="1.5" fill="currentColor"/></svg>
  </span></div>
  <div class="hole"></div>
"""

PEBBLE = "48% 52% 46% 54% / 55% 47% 53% 45%"
TINT = "#DDEEF3"
ACCENT = "#0088B0"
CARD = "#FBF9F4"
SHADOW = "42, 38, 34"

LAYOUTS = {
    "top": dict(
        wash_angle="180deg", galet_at="left: -320px; top: 900px;",
        copy_edge="top", copy_pad="0 0 36px", copy_justify="flex-end",
        phone_top=PHONE_TOP, phone_h=TILE_H, statusbar=STATUSBAR_HTML,
    ),
    "bottom": dict(
        wash_angle="0deg", galet_at="right: -300px; top: 200px;",
        copy_edge="bottom", copy_pad="0", copy_justify="center",
        phone_top=-FRAME, phone_h=TILE_H - PHONE_TOP + FRAME, statusbar="",
    ),
}


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


def check_copy(headline, subline):
    if len(headline.split()) > HEADLINE_MAX_WORDS:
        raise ValueError(f"headline has more than {HEADLINE_MAX_WORDS} words: {headline!r}")
    if len(subline.split()) > SUBLINE_MAX_WORDS:
        raise ValueError(f"subline has more than {SUBLINE_MAX_WORDS} words: {subline!r}")


def check_crop(crop, size):
    x0, y0, x1, y1 = crop
    if not (0 <= x0 < x1 <= size[0] and 0 <= y0 < y1 <= size[1]):
        raise ValueError(f"callout crop {crop} is empty or leaves the {size[0]}x{size[1]} capture")


def callout_rect(tile):
    x0, y0, x1, y1 = tile.crop
    w = round((x1 - x0) * SCALE * tile.zoom) + 2 * CALLOUT_BORDER
    h = round((y1 - y0) * SCALE * tile.zoom) + 2 * CALLOUT_BORDER
    sx0, sy0, sx1, sy1 = source_rect(tile)
    low, high = (PHONE_TOP, TILE_H) if tile.layout == "top" else (0, TILE_H - PHONE_TOP)
    x = min(max((sx0 + sx1 - w) // 2, CALLOUT_EDGE), TILE_W - CALLOUT_EDGE - w)
    y = min(max((sy0 + sy1 - h) // 2, low + CALLOUT_EDGE), high - CALLOUT_EDGE - h)
    return (x, y, x + w, y + h)


def source_rect(tile):
    x0, y0, x1, y1 = tile.crop
    left = PHONE_LEFT + FRAME
    top = PHONE_TOP + FRAME if tile.layout == "top" else 0
    shift = tile.crop_top
    sx0, sy0 = left + round(x0 * SCALE), top + round((y0 - shift) * SCALE)
    return (sx0, sy0, sx0 + round((x1 - x0) * SCALE), sy0 + round((y1 - y0) * SCALE))


def screen_box(layout):
    left = PHONE_LEFT + FRAME
    if layout == "top":
        return (left, PHONE_TOP + FRAME + max(STATUS_BAR_H, SCREEN_RADIUS), left + SCREEN_W, TILE_H)
    return (left, 0, left + SCREEN_W, TILE_H - PHONE_TOP - FRAME - SCREEN_RADIUS)


def check_headline_lines(headline, lines, last_line_words):
    if lines > HEADLINE_MAX_LINES:
        raise ValueError(f"headline runs on {lines} lines, the limit is {HEADLINE_MAX_LINES}: {headline!r}")
    if lines > 1 and last_line_words < 2:
        raise ValueError(f"headline leaves a single word alone on its last line: {headline!r}")


def typeset(text):
    return text.replace(" ?", " ?").replace(" :", " :")


def measure_headlines(chrome, headlines):
    """Per headline, (line count, words on the last line) as Chrome lays it out."""
    words = lambda h: "".join(f"<span>{html_escape.escape(w)}</span> " for w in typeset(h).split(" "))
    page = MEASURE_HTML.format(
        fonts=FONTS.replace(os.sep, "/"), w=TILE_W, side=COPY_SIDE, headline_css=HEADLINE_CSS,
        headlines="".join(f'<div class="headline">{words(h)}</div>' for h in headlines),
    )
    os.makedirs(WORK, exist_ok=True)
    src = os.path.join(WORK, "measure-headlines.html")
    with open(src, "w", encoding="utf-8") as f:
        f.write(page)
    dom = subprocess.run(
        [chrome, "--headless", "--disable-gpu", "--no-first-run", "--allow-file-access-from-files",
         "--virtual-time-budget=5000", "--dump-dom", src],
        capture_output=True, text=True, timeout=CAPTURE_TIMEOUT, check=True,
    ).stdout
    found = re.search(r'<pre id="out">(.*?)</pre>', dom, re.S)
    if not found or not found.group(1):
        raise RuntimeError("Chrome did not report the headline line boxes")
    return [tuple(m) for m in json.loads(html_escape.unescape(found.group(1)))]


def check_clean_cuts(ink, crop_top):
    if crop_top + VISIBLE_ROWS > len(ink):
        raise ValueError(f"the bleed ends at capture row {crop_top + VISIBLE_ROWS}, past the {len(ink)} rows")
    for y in (crop_top, crop_top + VISIBLE_ROWS):
        if 0 < y < len(ink) and (ink[y - 1] or ink[y]):
            raise ValueError(f"the bleed cuts a line of text at capture row {y}")


def compose(chrome, locale, tile, work):
    from PIL import Image

    name, headline, subline = tile.name, tile.headline, tile.subline
    check_copy(headline, subline)
    shot = capture_path(locale, tile.capture)
    store_module().assert_not_blank(shot)
    capture = Image.open(shot).convert("RGB")
    check_clean_cuts(ink_rows(capture), tile.crop_top)
    check_crop(tile.crop, capture.size)

    zoomed = SCALE * tile.zoom
    x0, y0, _, _ = tile.crop
    cx0, cy0, cx1, cy1 = callout_rect(tile)
    status_ink = INK if sum(capture.getpixel((CAPTURE_W // 2, 4))) > 384 else SAND
    page = TILE_HTML.format(
        title=html_escape.escape(f"{locale} {name}"),
        fonts=FONTS.replace(os.sep, "/"),
        shot=shot.replace(os.sep, "/"),
        w=TILE_W, h=TILE_H, band=PHONE_TOP, side=COPY_SIDE, sand=SAND, ink=INK, muted=MUTED,
        tint=TINT, accent=ACCENT, card=CARD, shadow=SHADOW, pebble=PEBBLE, status_ink=status_ink,
        headline_css=HEADLINE_CSS, status_h=STATUS_BAR_H, phone_radius=FRAME + SCREEN_RADIUS,
        phone_left=PHONE_LEFT, phone_w=PHONE_W, frame=FRAME, screen_w=SCREEN_W,
        crop=round(tile.crop_top * SCALE),
        callout_x=cx0, callout_y=cy0, callout_border=CALLOUT_BORDER,
        callout_w=cx1 - cx0 - 2 * CALLOUT_BORDER, callout_h=cy1 - cy0 - 2 * CALLOUT_BORDER,
        callout_bg_w=round(CAPTURE_W * zoomed), callout_bg_x=-round(x0 * zoomed), callout_bg_y=-round(y0 * zoomed),
        headline=html_escape.escape(typeset(headline)), subline=html_escape.escape(subline),
        **LAYOUTS[tile.layout],
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

    poster = Image.open(raw).convert("RGB")
    if poster.size != (TILE_W, TILE_H):
        raise ValueError(f"{locale}/{name}: Chrome wrote {poster.size}, not {TILE_W}x{TILE_H}")
    check_text_band(poster.transpose(Image.FLIP_TOP_BOTTOM) if tile.layout == "bottom" else poster)
    out = os.path.join(OUT, locale, f"{name}-{TILE_W}x{TILE_H}.png")
    poster.save(out, optimize=True)
    return out


def main():
    chrome = store_module().CHROME
    if not os.path.exists(chrome):
        sys.exit(f"headless Chrome not found at {chrome}")

    headlines = [tile.headline for tiles in TILES.values() for tile in tiles]
    for headline, (lines, last_words) in zip(headlines, measure_headlines(chrome, headlines)):
        check_headline_lines(headline, lines, last_words)

    for locale, tiles in TILES.items():
        work =os.path.join(WORK, locale)
        os.makedirs(work, exist_ok=True)
        os.makedirs(os.path.join(OUT, locale), exist_ok=True)
        for tile in tiles:
            path = compose(chrome, locale, tile, work)
            print(f"  {os.path.relpath(path, ROOT)}")


if __name__ == "__main__":
    main()
