"""Check every publication asset against what Play actually refuses.

Play rejects on format before a human ever looks at the listing, and the two
that bite are silent: an icon without an alpha channel, and a feature graphic
with one. This walks the whole set -- icons, feature graphic, screenshots,
fonts, and the legal pages' internal links -- and exits non-zero on the first
thing that would come back rejected.

Usage: python3 store/verify_assets.py
"""

import glob
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
        for name in ("ic_launcher", "ic_launcher_round"):
            path = os.path.join(RES, f"mipmap-{density}", f"{name}.png")
            check(Image.open(path).size == (px, px), f"mipmap-{density}/{name}.png is {px}x{px}")
    for density, px in ADAPTIVE_SIZES.items():
        for name in ("ic_launcher_foreground", "ic_launcher_monochrome"):
            path = os.path.join(RES, f"mipmap-{density}", f"{name}.png")
            image = Image.open(path)
            check(
                image.size == (px, px) and image.mode == "RGBA",
                f"mipmap-{density}/{name}.png is {px}x{px} with transparency",
            )
    for path in ("mipmap-anydpi-v26/ic_launcher.xml",
                 "mipmap-anydpi-v26/ic_launcher_round.xml",
                 "drawable/ic_launcher_background.xml"):
        check(os.path.exists(os.path.join(RES, path)), f"{path} exists")


def check_store_icon():
    print("store icon")
    path = rel("store", "icon", "ic_launcher-512.png")
    image = Image.open(path)
    check(image.size == (512, 512), f"512 icon is 512x512, got {image.size}")
    check(image.mode == "RGBA", f"512 icon keeps an alpha channel, got {image.mode}")
    check(os.path.getsize(path) < 1024 * 1024, "512 icon stays under 1 MB")


def check_feature_graphic():
    print("feature graphic")
    path = rel("store", "listing", "feature-graphic-1024x500.png")
    image = Image.open(path)
    check(image.size == (1024, 500), f"feature graphic is 1024x500, got {image.size}")
    check(image.mode == "RGB", f"feature graphic carries no alpha, got {image.mode}")


def check_screenshots():
    print("screenshots")
    for locale in ("fr", "en"):
        shots = sorted(glob.glob(rel("store", "listing", "screenshots", locale, "*.png")))
        check(len(shots) >= 2, f"{locale}: {len(shots)} screenshots (Play needs at least 2)")
        for path in shots:
            check(Image.open(path).size == (1080, 1920),
                  f"{locale}/{os.path.basename(path)} is 1080x1920")


def check_fonts():
    print("self-hosted fonts")
    for directory in (rel("app", "assets", "fonts"), rel("site", "fonts")):
        for name in ("SourceSerif4Variable-Roman.woff2", "SourceSerif4Variable-Italic.woff2"):
            path = os.path.join(directory, name)
            check(os.path.exists(path), os.path.relpath(path, ROOT))
            if os.path.exists(path):
                check(open(path, "rb").read(4) == b"wOF2",
                      f"{os.path.relpath(path, ROOT)} is a real woff2")
        check(os.path.exists(os.path.join(directory, "OFL.md")),
              os.path.relpath(os.path.join(directory, "OFL.md"), ROOT) + " (redistribution is required)")


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
    check_screenshots()
    check_fonts()
    check_site()
    print()
    if failures:
        print(f"{len(failures)} problem(s) would be rejected by Play")
        sys.exit(1)
    print("every asset matches what Play expects")


if __name__ == "__main__":
    main()
