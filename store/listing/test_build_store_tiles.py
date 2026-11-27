import importlib.util
import os
import unittest

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SAND = (244, 241, 234)
INK = (42, 38, 34)


def load():
    spec = importlib.util.spec_from_file_location(
        "build_store_tiles", os.path.join(HERE, "build_store_tiles.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sand_with_line(y0, y1):
    img = Image.new("RGB", (1080, 1920), SAND)
    img.paste(INK, (100, y0, 500, y1))
    return img


class TilesTable(unittest.TestCase):
    def test_six_tiles_per_locale_each_with_a_capture_and_an_int_crop_top(self):
        t = load()
        for locale in ("fr", "en"):
            self.assertEqual(6, len(t.TILES[locale]))
            for name, crop_top, headline, subline in t.TILES[locale]:
                self.assertIsInstance(crop_top, int, name)
                self.assertTrue(os.path.exists(t.capture_path(locale, name)), name)


class TextBandStaysWithin384(unittest.TestCase):
    def test_text_ending_on_row_383_passes(self):
        t = load()
        t.check_text_band(sand_with_line(300, 384))

    def test_text_reaching_row_384_raises(self):
        t = load()
        with self.assertRaises(ValueError):
            t.check_text_band(sand_with_line(300, 385))


class TheBleedNeverCutsALineOfText(unittest.TestCase):
    def bottom_cut(self, t, crop_top):
        return crop_top + t.VISIBLE_ROWS

    def test_a_line_straddling_the_bottom_cut_raises(self):
        t = load()
        cut = self.bottom_cut(t, 0)
        ink = t.ink_rows(sand_with_line(cut - 5, cut + 5))
        with self.assertRaises(ValueError):
            t.check_clean_cuts(ink, 0)

    def test_a_line_straddling_the_top_cut_raises(self):
        t = load()
        ink = t.ink_rows(sand_with_line(95, 105))
        with self.assertRaises(ValueError):
            t.check_clean_cuts(ink, 100)

    def test_both_cuts_in_gaps_pass(self):
        t = load()
        ink = t.ink_rows(sand_with_line(300, 340))
        t.check_clean_cuts(ink, 0)

    def test_every_real_capture_is_cut_cleanly_at_its_crop_top(self):
        t = load()
        for locale in ("fr", "en"):
            for name, crop_top, headline, subline in t.TILES[locale]:
                with self.subTest(locale=locale, tile=name):
                    ink = t.ink_rows(Image.open(t.capture_path(locale, name)))
                    t.check_clean_cuts(ink, crop_top)


if __name__ == "__main__":
    unittest.main()
