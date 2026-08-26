"""Render the Kayzen launcher and store icons from icon.html.

The artwork is typeset, not drawn: the mark is the letter K of Source Serif 4
followed by the cyan stamp that means "practised" in the Broadsheet system
(docs/functional/design/05-style-graphique.md). Keeping it as live text is what
lets the real typeface stay the source of truth -- an SVG would either need the
outlines extracted or a hand-drawn approximation of a licensed design.

Rendering goes through headless Chrome, then Pillow scales every target size
down from one oversized master so small sizes stay crisp.

Usage: python3 store/icon/build_icons.py
"""

import os
import subprocess
import sys

from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
HERE = os.path.join(ROOT, "store", "icon")
RES = os.path.join(ROOT, "app", "android", "res")
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
MASTER = 1600

PAPER = (0xF3, 0xF2, 0xF2, 255)

DENSITIES = {"mdpi": 1, "hdpi": 1.5, "xhdpi": 2, "xxhdpi": 3, "xxxhdpi": 4}

# @algo: the adaptive-icon safe zone is a 66dp circle inside the 108dp canvas,
# so content is scaled by its farthest opaque pixel from the centre rather than
# by its bounding box -- a box that fits the square still has corners outside
# the circle, and those corners are where the K's serifs live.
SAFE_ADAPTIVE = 66 / 108
SAFE_LEGACY = 0.84
SAFE_ROUND = 0.72
SAFE_STORE = 0.70


def render(html_path, out_path, size):
    subprocess.run(
        [
            CHROME, "--headless", "--disable-gpu",
            f"--screenshot={out_path}",
            f"--window-size={size},{size}",
            "--force-device-scale-factor=1",
            "--hide-scrollbars",
            "--default-background-color=00000000",
            html_path,
        ],
        capture_output=True,
        check=True,
    )
    return Image.open(out_path).convert("RGBA")


def max_radius(im):
    alpha = im.getchannel("A")
    cx, cy = im.width / 2, im.height / 2
    worst = 0.0
    px = alpha.load()
    for y in range(im.height):
        for x in range(im.width):
            if px[x, y] > 8:
                d = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
                if d > worst:
                    worst = d
    return worst


def fit(master, canvas, radius_fraction):
    bb = master.getbbox()
    art = master.crop(bb)
    side = max(art.size) * 2
    square = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    square.paste(art, ((side - art.width) // 2, (side - art.height) // 2), art)
    scale = (canvas / 2 * radius_fraction) / max_radius(square)
    target = max(1, round(side * scale))
    art = square.resize((target, target), Image.LANCZOS)
    out = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    off = (canvas - target) // 2
    out.paste(art, (off, off), art)
    return out


def on_paper(fg, circular=False):
    bg = Image.new("RGBA", fg.size, PAPER)
    if circular:
        mask = Image.new("L", fg.size, 0)
        ImageDraw.Draw(mask).ellipse((0, 0, fg.width - 1, fg.height - 1), fill=255)
        bg.putalpha(mask)
    bg.alpha_composite(fg)
    return bg


def blacken(im):
    out = im.copy()
    px = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = px[x, y]
            if a:
                px[x, y] = (0, 0, 0, a)
    return out


def write(im, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    im.save(path, optimize=True)
    print(f"  {os.path.relpath(path, ROOT)}  {im.size[0]}x{im.size[1]}")


def main():
    if not os.path.exists(CHROME):
        sys.exit(f"headless Chrome not found at {CHROME}")

    master = render(os.path.join(HERE, "icon.html"),
                    os.path.join(HERE, ".master.png"), MASTER)

    print("adaptive foreground + monochrome (108dp)")
    for name, factor in DENSITIES.items():
        canvas = round(108 * factor)
        fg = fit(master, canvas, SAFE_ADAPTIVE)
        write(fg, os.path.join(RES, f"mipmap-{name}", "ic_launcher_foreground.png"))
        write(blacken(fg), os.path.join(RES, f"mipmap-{name}", "ic_launcher_monochrome.png"))

    print("legacy square + round (48dp)")
    for name, factor in DENSITIES.items():
        canvas = round(48 * factor)
        write(on_paper(fit(master, canvas, SAFE_LEGACY)),
              os.path.join(RES, f"mipmap-{name}", "ic_launcher.png"))
        write(on_paper(fit(master, canvas, SAFE_ROUND), circular=True),
              os.path.join(RES, f"mipmap-{name}", "ic_launcher_round.png"))

    print("store listing icon")
    write(on_paper(fit(master, 512, SAFE_STORE)),
          os.path.join(HERE, "ic_launcher-512.png"))

    os.remove(os.path.join(HERE, ".master.png"))


if __name__ == "__main__":
    main()
