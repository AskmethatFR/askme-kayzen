"""Capture the six design screens from the real app, on a phone viewport.

These images illustrate `06-style-galets.md`. They used to be the designer's
prototype mock-ups; since the Galets refonte shipped screen by screen, the
prototype and the application drifted apart, so the documentation now shows
what the application actually renders.

The seeded demo board, the local server and the staging that drops the remote
font import all come from `store/listing/build_screenshots.py` -- imported
rather than copied, so the two capture sets can never disagree about what the
demo board contains.

The viewport is a Galaxy-S-class phone: 360x800 CSS pixels, the 20:9 format
Samsung has shipped since the S20. That is the frame the design was drawn for,
and the one the Android app actually gets -- a desktop window would only pad
the 620px screen column with sand paper on both sides. It differs from the
store's 360x640 because Play pins its phone screenshots to 16:9 while the
documentation is free to show the real proportions.

Getting 360dp out of Chrome takes the detour `build_screenshots.py` documents:
the window is clamped to a floor of 500 CSS pixels, so it is asked for at that
floor and the staged page's `html { zoom }` shrinks the CSS pixel back to a
phone's. The image Chrome writes is the window multiplied by the scale factor,
so the capture lands at 1000x2222 and is resampled down to 720x1600.

Prerequisite: dx build --platform web
Usage: python3 docs/functional/design/images/build_captures.py
"""

import importlib.util
import os
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
OUT = os.path.dirname(os.path.abspath(__file__))
VIEWPORT_W, VIEWPORT_H = 360, 800
CAPTURE_DPR = 2
OUT_W, OUT_H = VIEWPORT_W * 2, VIEWPORT_H * 2
CAPTURE_TIMEOUT = 90


def store_module():
    path = os.path.join(ROOT, "store", "listing", "build_screenshots.py")
    spec = importlib.util.spec_from_file_location("kayzen_store_captures", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def capture(bs, name, route):
    """Shoot one route, then kill the browser.

    Same completion signal as the store script: the ritual circle animates for
    ever, so Chrome writes the PNG when its virtual-time budget expires but
    does not exit. The file settling at a stable size is what says the capture
    is done.
    """
    out = os.path.join(OUT, f"{name}.png")
    if os.path.exists(out):
        os.remove(out)

    proc = subprocess.Popen(
        [
            bs.CHROME, "--headless", "--disable-gpu",
            "--no-first-run", "--no-default-browser-check",
            "--disable-background-networking", "--disable-sync",
            "--disable-default-apps", "--disable-extensions",
            f"--screenshot={out}",
            f"--window-size={bs.CHROME_MIN_WINDOW_W},{round(VIEWPORT_H * bs.ZOOM)}",
            f"--force-device-scale-factor={CAPTURE_DPR}",
            "--hide-scrollbars",
            "--virtual-time-budget=6000",
            "--lang=fr",
            "--accept-lang=fr-FR,fr",
            f"--user-data-dir={os.path.join(ROOT, 'target', 'docs-captures', '.chrome')}",
            f"http://127.0.0.1:{bs.PORT}{route}",
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
            raise TimeoutError(f"{name}: no screenshot after {CAPTURE_TIMEOUT}s")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()

    return out


def main():
    bs = store_module()
    if not os.path.isdir(bs.PUBLIC):
        sys.exit("no web build found -- run: dx build --platform web")
    if not os.path.exists(bs.CHROME):
        sys.exit(f"headless Chrome not found at {bs.CHROME}")

    from PIL import Image

    root = bs.stage("fr")
    httpd = bs.serve(root)
    try:
        for name, route in bs.SHOTS:
            path = capture(bs, name, route)
            Image.open(path).convert("RGB").resize(
                (OUT_W, OUT_H), Image.LANCZOS
            ).save(path, optimize=True)
            size = Image.open(path).size
            print(f"  {os.path.relpath(path, ROOT)}  {size[0]}x{size[1]}")
    finally:
        httpd.shutdown()
        httpd.server_close()


if __name__ == "__main__":
    main()
