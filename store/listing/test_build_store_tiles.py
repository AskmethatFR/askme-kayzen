import importlib.util
import os
import tempfile
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


APPROVED = {
    "fr": [
        ("01-aujourdhui", "01-aujourdhui", 0, "Un petit pas suffit.", "Pas de série, pas de pression."),
        ("02-detail", "02-detail", 0, "Un jour sans ? Rien ne casse.", "Aucune série à protéger."),
        ("03-semaine", "04-semaine", 0, "Votre rythme, pas le nôtre.", "Plus ou moins : vous décidez."),
        ("04-rituel", "03-rituel", 0, "Un moment pour vous.", "Arrêter n'est pas échouer."),
        ("05-ancrees", "05-ancrees", 0, "Devenue naturelle ? Ancrez-la.", "Elle libère une place."),
        ("06-ajouter", "06-ajouter", 0, "Commencez tout petit.", "Cinq habitudes au plus."),
    ],
    "en": [
        ("01-aujourdhui", "01-aujourdhui", 0, "One small step is enough.", "No streaks, no pressure."),
        ("02-detail", "02-detail", 0, "Skip a day. Nothing breaks.", "No streak to protect."),
        ("03-semaine", "04-semaine", 0, "Your pace, not ours.", "More or less: you decide."),
        ("04-rituel", "03-rituel", 0, "A moment for yourself.", "Stopping isn't failing."),
        ("05-ancrees", "05-ancrees", 0, "Became natural? Anchor it.", "It frees up a seat."),
        ("06-ajouter", "06-ajouter", 0, "Start tiny.", "Five habits at most."),
    ],
}


class TilesTable(unittest.TestCase):
    def test_six_tiles_per_locale_each_with_a_capture_and_an_int_crop_top(self):
        t = load()
        for locale in ("fr", "en"):
            self.assertEqual(6, len(t.TILES[locale]))
            for name, capture, crop_top, headline, subline in t.TILES[locale]:
                self.assertIsInstance(crop_top, int, name)
                self.assertTrue(os.path.exists(t.capture_path(locale, capture)), name)

    def test_the_table_is_the_approved_copy_in_the_approved_order(self):
        t = load()
        for locale in ("fr", "en"):
            with self.subTest(locale=locale):
                self.assertEqual(APPROVED[locale], t.TILES[locale])


class CopyStaysShort(unittest.TestCase):
    def test_a_headline_of_five_words_passes(self):
        t = load()
        t.check_copy("Un petit pas suffit ici.", "Pas de série.")

    def test_a_headline_of_eight_words_raises(self):
        t = load()
        with self.assertRaises(ValueError):
            t.check_copy("Un petit pas suffit ici pour vous aujourd'hui.", "Pas de série.")

    def test_a_subline_of_six_words_passes(self):
        t = load()
        t.check_copy("Un petit pas.", "Pas de série, pas de pression.")

    def test_a_subline_of_seven_words_raises(self):
        t = load()
        with self.assertRaises(ValueError):
            t.check_copy("Un petit pas.", "Pas de série, pas de pression, jamais.")


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

    def test_a_crop_whose_bleed_runs_past_the_capture_raises(self):
        t = load()
        ink = [False] * 1920
        t.check_clean_cuts(ink, 18)
        with self.assertRaises(ValueError):
            t.check_clean_cuts(ink, 19)

    def test_a_line_ending_on_the_row_just_above_the_bottom_cut_raises(self):
        t = load()
        cut = self.bottom_cut(t, 0)
        ink = t.ink_rows(sand_with_line(cut - 5, cut))
        with self.assertRaises(ValueError):
            t.check_clean_cuts(ink, 0)

    def test_a_line_ending_on_the_row_just_above_the_top_cut_raises(self):
        t = load()
        ink = t.ink_rows(sand_with_line(10, 18))
        with self.assertRaises(ValueError):
            t.check_clean_cuts(ink, 18)

    def test_a_two_pixel_speck_is_not_a_line_of_text_but_three_pixels_are(self):
        t = load()
        for width, is_ink in ((2, False), (3, True)):
            with self.subTest(width=width):
                img = Image.new("RGB", (1080, 1920), SAND)
                img.paste(INK, (100, 500, 100 + width, 501))
                self.assertEqual(t.ink_rows(img)[500], is_ink)

    def test_every_real_capture_is_cut_cleanly_at_its_crop_top(self):
        t = load()
        for locale in ("fr", "en"):
            for name, capture, crop_top, headline, subline in t.TILES[locale]:
                with self.subTest(locale=locale, tile=name):
                    ink = t.ink_rows(Image.open(t.capture_path(locale, capture)))
                    t.check_clean_cuts(ink, crop_top)


class ComposeRefusesABlankCapture(unittest.TestCase):
    def test_a_blank_capture_never_becomes_a_tile(self):
        t = load()
        with tempfile.TemporaryDirectory() as tmp:
            blank = os.path.join(tmp, "blank.png")
            Image.new("RGB", (1080, 1920), "white").save(blank)
            os.makedirs(os.path.join(tmp, "out", "fr"))
            t.OUT = os.path.join(tmp, "out")
            t.capture_path = lambda locale, name: blank
            t.run_chrome = lambda chrome, args, raw, label: Image.new(
                "RGB", (t.TILE_W, t.TILE_H), SAND
            ).save(raw)
            with self.assertRaises(RuntimeError):
                t.compose("chrome", "fr", "01-x", "c", 0, "Hi", "there", tmp)
            self.assertEqual([], os.listdir(os.path.join(tmp, "out", "fr")))


class TheCopyBlockSitsAtTheSameHeightWhateverTheHeadlineLength(unittest.TestCase):
    def test_every_real_tile_ends_its_copy_within_a_few_rows_of_the_others(self):
        t = load()
        ends = {}
        for locale in ("fr", "en"):
            for name, *_ in t.TILES[locale]:
                tile = Image.open(os.path.join(t.OUT, locale, f"{name}-1080x1920.png"))
                ink = t.ink_rows(tile.crop((0, 0, t.TILE_W, t.PHONE_TOP)))
                ends[f"{locale}/{name}"] = max(y for y, on in enumerate(ink) if on)
        spread = max(ends.values()) - min(ends.values())
        self.assertLessEqual(spread, 16, ends)


if __name__ == "__main__":
    unittest.main()
