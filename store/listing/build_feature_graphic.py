"""Render the two 1024x500 Play Store feature graphics, one per locale.

Galets style (docs/functional/design/06-style-galets.md): a sand ground, the
Fraunces "Kayzen" with the cyan dot of the icon, one Figtree line under it,
centred in the middle ~70%. Filled cyan pebbles (practised) and outlined
pebbles (not yet) sit near the two edges only, in varied sizes like the app's
staircase. No phone: Play crops the edges and warns against device imagery.
PEBBLES rows are (x, y, w, h, filled), all within the outer 15% of a side.

Play rejects a feature graphic that carries an alpha channel, so the output is
flattened to RGB on purpose.

Usage: python3 store/listing/build_feature_graphic.py
"""

import html as html_escape
import importlib.util
import os
import sys

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
HERE = os.path.join(ROOT, "store", "listing")
WORK = os.path.join(ROOT, "target", "store-feature-graphic")
FONTS = os.path.join(ROOT, "app", "assets", "fonts")
WIDTH, HEIGHT = 1024, 500

SAND = "#F4F1EA"
INK = "#2A2622"
CYAN = "#0088b0"
OUTLINE = "#8A8177"

LINES = {"fr": "Petits pas, sans série.", "en": "Small steps, no streaks."}
ALT = {
    "fr": "Le nom Kayzen et la phrase « Petits pas, sans série. » sur un papier sable, "
          "entre des galets cyan pleins et des galets en contour.",
    "en": "The Kayzen name and the line \"Small steps, no streaks.\" on sand-coloured paper, "
          "between filled cyan pebbles and outlined pebbles.",
}

PEBBLES = [
    (28, 60, 56, 50, True),
    (92, 118, 34, 30, False),
    (30, 190, 92, 80, True),
    (100, 292, 40, 36, False),
    (24, 340, 64, 56, False),
    (84, 420, 44, 40, True),
    (936, 54, 44, 40, False),
    (884, 126, 70, 62, True),
    (950, 216, 40, 36, True),
    (890, 270, 96, 84, False),
    (960, 380, 36, 32, True),
    (894, 410, 56, 50, True),
]

HTML = """<!doctype html>
<meta charset="utf-8">
<title>Kayzen feature graphic {locale}</title>
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
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
  }}
  .wordmark {{
    font-family: 'Fraunces', Georgia, serif;
    font-size: 150px;
    line-height: 1;
    color: {ink};
    font-variation-settings: 'wght' 620, 'opsz' 144, 'SOFT' 30;
  }}
  .dot {{ color: {cyan}; }}
  .line {{
    font-family: 'Figtree', system-ui, sans-serif;
    font-size: 44px;
    font-weight: 500;
    color: {ink};
    margin-top: 18px;
  }}
  .pebble {{
    position: absolute;
    box-sizing: border-box;
    border-radius: 48% 52% 46% 54% / 55% 47% 53% 45%;
  }}
  .filled {{ background: {cyan}; }}
  .hollow {{ border: 4px solid {outline}; }}
</style>
<div class="wordmark">Kayzen<span class="dot">.</span></div>
<div class="line">{line}</div>
{pebbles}
"""


def output_path(locale):
    return os.path.join(HERE, f"feature-graphic-{locale}-{WIDTH}x{HEIGHT}.png")


def page_html(locale):
    pebbles = "\n".join(
        f'<div class="pebble {"filled" if filled else "hollow"}" '
        f'style="left:{x}px;top:{y}px;width:{w}px;height:{h}px"></div>'
        for x, y, w, h, filled in PEBBLES
    )
    return HTML.format(
        locale=locale, fonts=FONTS.replace(os.sep, "/"), w=WIDTH, h=HEIGHT,
        sand=SAND, ink=INK, cyan=CYAN, outline=OUTLINE,
        line=html_escape.escape(LINES[locale]), pebbles=pebbles,
    )


def tiles_module():
    spec = importlib.util.spec_from_file_location(
        "kayzen_store_tiles", os.path.join(HERE, "build_store_tiles.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    tiles = tiles_module()
    chrome = tiles.store_module().CHROME
    if not os.path.exists(chrome):
        sys.exit(f"headless Chrome not found at {chrome}")

    os.makedirs(WORK, exist_ok=True)
    for locale in LINES:
        src = os.path.join(WORK, f"{locale}.html")
        raw = os.path.join(WORK, f"{locale}.png")
        open(src, "w", encoding="utf-8").write(page_html(locale))
        if os.path.exists(raw):
            os.remove(raw)
        tiles.run_chrome(
            chrome,
            [f"--window-size={WIDTH},{HEIGHT}", "--force-device-scale-factor=1",
             "--allow-file-access-from-files", src],
            raw,
            f"{locale} feature graphic",
        )
        img = Image.open(raw).convert("RGB")
        if img.size != (WIDTH, HEIGHT):
            raise ValueError(f"{locale}: Chrome wrote {img.size}, not {WIDTH}x{HEIGHT}")
        img.save(output_path(locale), optimize=True)
        print(f"  {os.path.relpath(output_path(locale), ROOT)}  {WIDTH}x{HEIGHT}")


if __name__ == "__main__":
    main()
