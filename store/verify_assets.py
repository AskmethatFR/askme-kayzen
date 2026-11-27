"""Check every publication asset against what Play actually refuses.

Play rejects on format before a human ever looks at the listing, and the two
that bite are silent: an icon without an alpha channel, and a feature graphic
with one. This walks the whole set -- icons, feature graphic, phone tiles, alt text,
fonts, and the legal pages' internal links -- and exits non-zero on the first
thing that would come back rejected.

Usage: python3 store/verify_assets.py
"""

import glob
import importlib.util
import os
import re
import sys

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
RES = os.path.join(ROOT, "app", "android", "res")

LEGACY_SIZES = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
ADAPTIVE_SIZES = {"mdpi": 108, "hdpi": 162, "xhdpi": 216, "xxhdpi": 324, "xxxhdpi": 432}

failures = []


def check(condition, message):
    print(("  ok    " if condition else "  FAIL  ") + message)
    if not condition:
        failures.append(message)


def rel(*parts):
    return os.path.join(ROOT, *parts)


def check_icons():
    print("launcher icons")
    for density, px in LEGACY_SIZES.items():
        path = os.path.join(RES, f"mipmap-{density}", "ic_launcher.png")
        check(Image.open(path).size == (px, px), f"mipmap-{density}/ic_launcher.png is {px}x{px}")
    for density, px in ADAPTIVE_SIZES.items():
        for name in ("ic_launcher_foreground", "ic_launcher_monochrome"):
            path = os.path.join(RES, f"mipmap-{density}", f"{name}.png")
            image = Image.open(path)
            check(
                image.size == (px, px) and image.mode == "RGBA",
                f"mipmap-{density}/{name}.png is {px}x{px} with transparency",
            )
    for path in ("mipmap-anydpi-v26/ic_launcher.xml",
                 "drawable/ic_launcher_background.xml"):
        check(os.path.exists(os.path.join(RES, path)), f"{path} exists")


def check_store_icon():
    print("store icon")
    path = rel("store", "icon", "ic_launcher-512.png")
    image = Image.open(path)
    check(image.size == (512, 512), f"512 icon is 512x512, got {image.size}")
    check(image.mode == "RGBA", f"512 icon keeps an alpha channel, got {image.mode}")
    check(os.path.getsize(path) < 1024 * 1024, "512 icon stays under 1 MB")


def tile_problems(path):
    problems = []
    with Image.open(path) as image:
        width, height = image.size
        if image.mode != "RGB":
            problems.append(f"{image.mode}, Play needs RGB with no alpha")
    if not all(320 <= side <= 3840 for side in (width, height)):
        problems.append(f"{width}x{height}, each side must be 320-3840 px")
    if 10 * max(width, height) > 23 * min(width, height):
        problems.append(f"{width}x{height}, long side over 2.3 times the short side")
    size = os.path.getsize(path)
    if size > 8_000_000:
        problems.append(f"{size / 1_000_000:.1f} MB, the limit is 8 MB")
    return problems


def listing_module(name):
    """Load a store/listing script, so the layout and the tile table have one owner."""
    spec = importlib.util.spec_from_file_location(
        name, os.path.join(ROOT, "store", "listing", f"{name}.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tiles = listing_module("build_store_tiles")
TILES = tiles.TILES
BLANK_MAX_COLOURS = listing_module("build_screenshots").BLANK_MAX_COLOURS


def blank_phone_problems(path, tile):
    """An empty phone under a headline passes every format check, so look inside the frame.

    The status bar is outside the box, and the callout with its shadow is painted over: a
    callout cropped from the same capture must not make a blank screen look busy.
    """
    cx0, cy0, cx1, cy1 = tiles.callout_rect(tile)
    reach = tiles.SHADOW_REACH
    with Image.open(path) as image:
        tile_image = image.convert("RGB")
    tile_image.paste((0, 0, 0), (cx0 - reach, cy0 - reach, cx1 + reach, cy1 + reach))
    screen = tile_image.crop(tiles.screen_box(tile.layout))
    if screen.getcolors(maxcolors=BLANK_MAX_COLOURS) is not None:
        return [f"the phone shows a blank screen (<= {BLANK_MAX_COLOURS} colours)"]
    return []


def tile_name_problems(locale, names):
    expected = {f"{name}-{tiles.TILE_W}x{tiles.TILE_H}.png" for name, *_ in TILES[locale]}
    return [f"{name} is not in the tile table" for name in sorted(set(names) - expected)] + [
        f"{name} is missing" for name in sorted(expected - set(names))
    ]


def tile_set_problems(paths):
    problems = []
    if not 4 <= len(paths) <= 8:
        problems.append(f"{len(paths)} tiles, Play takes 4 to 8")
    at_promotion = 0
    for path in paths:
        with Image.open(path) as image:
            at_promotion += min(image.size) >= 1080
    if at_promotion < 4:
        problems.append(f"{at_promotion} tiles at 1080 px or more, promotion needs 4")
    return problems


def feature_graphic_problems(path):
    problems = []
    with Image.open(path) as image:
        if image.size != (1024, 500):
            problems.append(f"{image.size[0]}x{image.size[1]}, must be 1024x500")
        if image.mode != "RGB":
            problems.append(f"{image.mode}, no alpha allowed")
    return problems


def alt_texts(markdown):
    texts = []
    for line in markdown.splitlines():
        if not line.startswith("|"):
            continue
        label, _, text = line.strip().strip("|").partition("|")
        label = label.strip().lower()
        if "alt text" in label or "texte alternatif" in label:
            texts.append(text.strip())
    return texts


def alt_problems(text):
    problems = []
    if len(text) > 140:
        problems.append(f"{len(text)} characters, the limit is 140")
    if text.lower().startswith(("image of", "image de")):
        problems.append("starts with 'image of' or 'image de'")
    return problems


def report(problems, label, ok_text):
    check(not problems, f"{label}: " + ("; ".join(problems) if problems else ok_text))


def check_feature_graphic():
    print("feature graphic")
    for locale in ("fr", "en"):
        path = rel("store", "listing", f"feature-graphic-{locale}-1024x500.png")
        report(feature_graphic_problems(path), f"{locale} feature graphic", "1024x500 RGB")


def check_tiles():
    print("phone tiles")
    for locale in ("fr", "en"):
        paths = sorted(glob.glob(rel("store", "listing", "tiles", locale, "*.png")))
        report(tile_set_problems(paths), f"{locale}: {len(paths)} tiles",
               "4 to 8, at least 4 at 1080 px")
        report(tile_name_problems(locale, [os.path.basename(p) for p in paths]),
               f"{locale} tile files", "exactly the tiles in the table")
        by_file = {f"{t.name}-{tiles.TILE_W}x{tiles.TILE_H}.png": t for t in TILES[locale]}
        for path in paths:
            if os.path.basename(path) not in by_file:
                continue
            report(tile_problems(path) + blank_phone_problems(path, by_file[os.path.basename(path)]),
                   f"{locale}/{os.path.basename(path)}",
                   "RGB, sides, ratio and size within Play's limits, phone shows a screen")


def check_alt_text():
    print("feature graphic alt text")
    for locale in ("fr", "en"):
        with open(rel("store", "listing", f"copy.{locale}.md"), encoding="utf-8") as f:
            texts = alt_texts(f.read())
        if not texts:
            check(False, f"{locale}: no alt text row in copy.{locale}.md")
        for text in texts:
            report(alt_problems(text), f"{locale} alt text ({len(text)} characters)",
                   "140 or fewer, no 'image of' opener")


REQUIRED_FONTS = {
    os.path.join("app", "assets", "fonts"): ("Fraunces-Variable.woff2", "Figtree-Variable.woff2"),
    os.path.join("site", "fonts"): ("SourceSerif4Variable-Roman.woff2", "SourceSerif4Variable-Italic.woff2"),
}


def font_problems(directory, names):
    problems = []
    for name in names:
        path = os.path.join(directory, name)
        if not os.path.exists(path):
            problems.append(f"{name} is missing")
        else:
            with open(path, "rb") as f:
                if f.read(4) != b"wOF2":
                    problems.append(f"{name} is not a real woff2")
    return problems


def check_fonts():
    print("self-hosted fonts")
    for directory, names in REQUIRED_FONTS.items():
        report(font_problems(rel(directory), names), directory, "fonts present, real woff2")
        check(os.path.exists(rel(directory, "OFL.md")),
              f"{directory}/OFL.md (redistribution is required)")


def check_site():
    print("legal pages")
    for page in sorted(glob.glob(rel("site", "*.html"))):
        html = open(page, encoding="utf-8").read()
        name = os.path.basename(page)
        for href in re.findall(r'href="([^"]+)"', html):
            if href.startswith(("mailto:", "http")):
                continue
            check(os.path.exists(rel("site", href.split("#")[0])), f"{name} -> {href}")
        check("fonts.googleapis" not in html, f"{name} makes no remote font request")


def main():
    check_icons()
    check_store_icon()
    check_feature_graphic()
    check_tiles()
    check_alt_text()
    check_fonts()
    check_site()
    print()
    if failures:
        print(f"{len(failures)} problem(s) would be rejected by Play")
        sys.exit(1)
    print("every asset matches what Play expects")


if __name__ == "__main__":
    main()
