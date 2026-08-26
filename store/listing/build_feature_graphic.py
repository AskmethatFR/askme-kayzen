"""Render the 1024x500 Play Store feature graphic.

Same Broadsheet masthead as the app: Source Serif 4 ink on paper, the cyan
stamp as the only spot colour, a 2px ink rule, and nothing else.

Play rejects a feature graphic that carries an alpha channel, so the output is
flattened to RGB on purpose.

Usage: python3 store/listing/build_feature_graphic.py
"""

import os
import subprocess
import sys

from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
HERE = os.path.join(ROOT, "store", "listing")
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
SCALE = 2
WIDTH, HEIGHT = 1024, 500

HTML = """<!doctype html>
<title>Kayzen feature graphic</title>
<style>
  @font-face {{
    font-family: 'Source Serif 4';
    src: url('../../app/assets/fonts/SourceSerif4Variable-Roman.woff2') format('woff2');
    font-weight: 200 900;
  }}
  @font-face {{
    font-family: 'Source Serif 4';
    src: url('../../app/assets/fonts/SourceSerif4Variable-Italic.woff2') format('woff2');
    font-weight: 200 900;
    font-style: italic;
  }}
  html, body {{ margin: 0; padding: 0; }}
  body {{
    width: {w}px; height: {h}px;
    background: #f3f2f2;
    font-family: 'Source Serif 4', Georgia, serif;
    color: #201e1d;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
  }}
  .kicker {{
    font-size: {kicker}px;
    letter-spacing: {track}px;
    text-transform: uppercase;
    color: #6f6a68;
    margin-bottom: {gap1}px;
  }}
  .wordmark {{
    font-size: {word}px;
    line-height: 1;
    font-variation-settings: 'wght' 660, 'opsz' 60;
  }}
  .stamp {{ color: #0088b0; }}
  .rule {{
    width: {rule}px; height: {ruleh}px;
    background: #201e1d;
    margin: {gap2}px 0 {gap3}px;
  }}
  .tagline {{
    font-style: italic;
    font-size: {tag}px;
    color: #201e1d;
    font-variation-settings: 'wght' 400, 'opsz' 40;
  }}
</style>
<div class="kicker">Une habitude à la fois</div>
<div class="wordmark">Kayzen<span class="stamp">.</span></div>
<div class="rule"></div>
<div class="tagline">Pas de série. Pas de rouge. Pas de pression.</div>
"""


def main():
    if not os.path.exists(CHROME):
        sys.exit(f"headless Chrome not found at {CHROME}")

    w, h = WIDTH * SCALE, HEIGHT * SCALE
    html = HTML.format(
        w=w, h=h,
        kicker=int(19 * SCALE), track=int(4.5 * SCALE), gap1=int(26 * SCALE),
        word=int(112 * SCALE), rule=int(300 * SCALE), ruleh=int(2 * SCALE),
        gap2=int(30 * SCALE), gap3=int(22 * SCALE), tag=int(27 * SCALE),
    )
    src = os.path.join(HERE, ".feature-graphic.html")
    raw = os.path.join(HERE, ".feature-graphic.png")
    open(src, "w").write(html)
    subprocess.run(
        [CHROME, "--headless", "--disable-gpu", f"--screenshot={raw}",
         f"--window-size={w},{h}", "--force-device-scale-factor=1",
         "--hide-scrollbars", src],
        capture_output=True, check=True,
    )

    out = os.path.join(HERE, "feature-graphic-1024x500.png")
    Image.open(raw).convert("RGB").resize((WIDTH, HEIGHT), Image.LANCZOS).save(out, optimize=True)
    os.remove(src)
    os.remove(raw)
    print(f"  {os.path.relpath(out, ROOT)}  {WIDTH}x{HEIGHT}")


if __name__ == "__main__":
    main()
