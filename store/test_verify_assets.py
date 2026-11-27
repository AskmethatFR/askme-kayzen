"""Tests for store/verify_assets.py: each Play rule refuses a bad asset.

Run: python3 -m unittest discover -s store -p 'test_verify_assets.py'
"""

import glob
import importlib.util
import os
import shutil
import tempfile
import unittest

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SAND = (244, 241, 234)


def load():
    spec = importlib.util.spec_from_file_location(
        "verify_assets", os.path.join(HERE, "verify_assets.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


va = load()


class Scratch(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir)

    def image(self, name, size=(1080, 1920), mode="RGB", pad_to=None):
        path = os.path.join(self.dir, name)
        Image.new(mode, size, SAND if mode == "RGB" else (0, 0, 0, 0)).save(path, "PNG")
        if pad_to is not None:
            with open(path, "ab") as f:
                f.write(b"\0" * (pad_to - os.path.getsize(path)))
        return path


class TileRules(Scratch):
    def test_passing_tile_has_no_problems(self):
        self.assertEqual(va.tile_problems(self.image("ok.png")), [])

    def test_rgba_tile_is_refused(self):
        problems = va.tile_problems(self.image("rgba.png", mode="RGBA"))
        self.assertTrue(any("RGB" in p for p in problems), problems)

    def test_side_range_bounds(self):
        cases = [((319, 640), False), ((320, 640), True),
                 ((3840, 1920), True), ((3841, 1920), False)]
        for size, ok in cases:
            with self.subTest(size=size):
                self.assertEqual(not va.tile_problems(self.image("s.png", size=size)), ok)

    def test_ratio_limit_is_2_3(self):
        cases = [((1080, 2484), True), ((1080, 2592), False)]
        for size, ok in cases:
            with self.subTest(size=size):
                self.assertEqual(not va.tile_problems(self.image("r.png", size=size)), ok)

    def test_size_limit_is_8_mb(self):
        cases = [(8_000_000, True), (8_000_001, False), (9_000_000, False)]
        for size, ok in cases:
            with self.subTest(size=size):
                path = self.image("big.png", pad_to=size)
                self.assertEqual(not va.tile_problems(path), ok)


class TileSetRules(Scratch):
    def tiles(self, count, size=(1080, 1920)):
        return [self.image(f"t{i}.png", size=size) for i in range(count)]

    def test_count_must_be_four_to_eight(self):
        cases = [(3, False), (4, True), (8, True), (9, False)]
        for count, ok in cases:
            with self.subTest(count=count):
                self.assertEqual(not va.tile_set_problems(self.tiles(count)), ok)

    def test_promotion_needs_four_tiles_at_1080(self):
        paths = self.tiles(3) + [self.image("small.png", size=(540, 960))]
        problems = va.tile_set_problems(paths)
        self.assertTrue(any("1080" in p for p in problems), problems)

    def test_promotion_boundary_is_1080_on_the_short_side(self):
        cases = [
            (self.tiles(3) + [self.image("just-under.png", size=(1079, 1919))], False),
            (self.tiles(4), True),
        ]
        for paths, ok in cases:
            with self.subTest(count=len(paths)):
                self.assertEqual(not va.tile_set_problems(paths), ok)


class FeatureGraphicRules(Scratch):
    def test_1024x500_rgb_passes(self):
        self.assertEqual(va.feature_graphic_problems(self.image("f.png", size=(1024, 500))), [])

    def test_wrong_size_is_refused(self):
        problems = va.feature_graphic_problems(self.image("f.png", size=(1000, 500)))
        self.assertTrue(problems)

    def test_alpha_is_refused(self):
        problems = va.feature_graphic_problems(self.image("f.png", size=(1024, 500), mode="RGBA"))
        self.assertTrue(problems)


class AltTextRules(unittest.TestCase):
    def test_140_characters_pass_and_141_fail(self):
        self.assertEqual(va.alt_problems("a" * 140), [])
        self.assertTrue(va.alt_problems("a" * 141))

    def test_image_prefix_is_refused_case_insensitively(self):
        cases = [("Image of a habit tracker", False), ("Image de l'ecran", False),
                 ("IMAGE DE l'ecran", False), ("Une image de l'ecran", True)]
        for text, ok in cases:
            with self.subTest(text=text):
                self.assertEqual(not va.alt_problems(text), ok)


class AltTextParsing(unittest.TestCase):
    def test_reads_feature_graphic_row_in_both_languages(self):
        en = "| Feature graphic alt text (140 characters max) | Hello. |\n"
        fr = "| Texte alternatif du feature graphic (140 caractères max) | Bonjour. |\n"
        self.assertEqual(va.alt_texts(en), ["Hello."])
        self.assertEqual(va.alt_texts(fr), ["Bonjour."])

    def test_no_alt_row_gives_nothing(self):
        self.assertEqual(va.alt_texts("| App category | Productivity |\n"), [])


class FontRules(Scratch):
    def font(self, name, magic=b"wOF2"):
        with open(os.path.join(self.dir, name), "wb") as f:
            f.write(magic + b"\0" * 8)

    def test_present_woff2_passes(self):
        self.font("A.woff2")
        self.assertEqual(va.font_problems(self.dir, ("A.woff2",)), [])

    def test_missing_font_is_refused(self):
        self.assertTrue(va.font_problems(self.dir, ("A.woff2",)))

    def test_file_that_is_not_woff2_is_refused(self):
        self.font("A.woff2", magic=b"nope")
        self.assertTrue(va.font_problems(self.dir, ("A.woff2",)))

    def test_app_fonts_are_the_ones_the_graphics_use(self):
        self.assertEqual(
            sorted(va.REQUIRED_FONTS[os.path.join("app", "assets", "fonts")]),
            ["Figtree-Variable.woff2", "Fraunces-Variable.woff2"],
        )


class RealAssets(unittest.TestCase):
    def test_real_fonts_pass(self):
        for directory, names in va.REQUIRED_FONTS.items():
            with self.subTest(directory=directory):
                self.assertEqual(va.font_problems(os.path.join(va.ROOT, directory), names), [])

    def test_real_tiles_pass(self):
        for locale in ("fr", "en"):
            with self.subTest(locale=locale):
                paths = sorted(glob.glob(os.path.join(va.ROOT, "store", "listing", "tiles", locale, "*.png")))
                self.assertEqual(va.tile_set_problems(paths), [])
                for path in paths:
                    self.assertEqual(va.tile_problems(path), [], path)

    def test_real_feature_graphics_pass(self):
        for locale in ("fr", "en"):
            with self.subTest(locale=locale):
                path = os.path.join(va.ROOT, "store", "listing", f"feature-graphic-{locale}-1024x500.png")
                self.assertEqual(va.feature_graphic_problems(path), [])

    def test_real_alt_texts_pass(self):
        for locale in ("fr", "en"):
            with self.subTest(locale=locale):
                with open(os.path.join(va.ROOT, "store", "listing", f"copy.{locale}.md"), encoding="utf-8") as f:
                    texts = va.alt_texts(f.read())
                self.assertTrue(texts)
                for text in texts:
                    self.assertEqual(va.alt_problems(text), [])


if __name__ == "__main__":
    unittest.main()
