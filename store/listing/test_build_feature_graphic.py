import importlib.util
import os
import unittest

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SAND = (244, 241, 234)
CYAN = (0, 136, 176)

LINES = {"fr": "Petits pas, sans série.", "en": "Small steps, no streaks."}
ALT = {
    "fr": "Le nom Kayzen et la phrase « Petits pas, sans série. » sur un papier sable, "
          "entre des galets cyan pleins et des galets en contour.",
    "en": "The Kayzen name and the line \"Small steps, no streaks.\" on sand-coloured paper, "
          "between filled cyan pebbles and outlined pebbles.",
}


def load():
    spec = importlib.util.spec_from_file_location(
        "build_feature_graphic", os.path.join(HERE, "build_feature_graphic.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class OneGraphicPerLocale(unittest.TestCase):
    def test_each_locale_has_its_own_file_beside_the_listing(self):
        g = load()
        for locale in ("fr", "en"):
            self.assertEqual(
                os.path.join(HERE, f"feature-graphic-{locale}-1024x500.png"),
                g.output_path(locale),
            )

    def test_the_old_shared_graphic_is_gone(self):
        self.assertFalse(os.path.exists(os.path.join(HERE, "feature-graphic-1024x500.png")))


class TheApprovedText(unittest.TestCase):
    def test_the_line_is_the_approved_one_per_locale(self):
        g = load()
        self.assertEqual(LINES, g.LINES)

    def test_the_page_carries_the_name_the_line_the_fonts_and_no_picture(self):
        g = load()
        for locale, line in LINES.items():
            with self.subTest(locale=locale):
                page = g.page_html(locale)
                self.assertIn(line, page)
                self.assertIn("Kayzen", page)
                self.assertIn("Fraunces", page)
                self.assertIn("Figtree", page)
                self.assertIn("#0088b0", page)
                self.assertNotIn("<img", page)

    def test_alt_text_is_the_approved_one_per_locale(self):
        g = load()
        self.assertEqual(ALT, g.ALT)

    def test_alt_text_respects_play_limits(self):
        g = load()
        for locale, alt in g.ALT.items():
            with self.subTest(locale=locale):
                self.assertLessEqual(len(alt), 140)
                self.assertFalse(alt.lower().startswith(("image of", "image de")))


class PebblesStayAtTheEdges(unittest.TestCase):
    def test_every_pebble_is_inside_the_canvas_and_outside_the_central_70_percent(self):
        g = load()
        self.assertTrue(g.PEBBLES)
        left, right = g.WIDTH * 0.15, g.WIDTH * 0.85
        for x, y, w, h, filled in g.PEBBLES:
            with self.subTest(pebble=(x, y)):
                self.assertTrue(0 <= x and x + w <= g.WIDTH and 0 <= y and y + h <= g.HEIGHT)
                self.assertTrue(x + w <= left or x >= right)

    def test_each_side_has_filled_and_hollow_pebbles_of_varied_sizes(self):
        g = load()
        for side in (lambda x: x < g.WIDTH / 2, lambda x: x >= g.WIDTH / 2):
            mine = [p for p in g.PEBBLES if side(p[0])]
            self.assertTrue(any(p[4] for p in mine))
            self.assertTrue(any(not p[4] for p in mine))
            self.assertGreater(len({p[2] for p in mine}), 2)


class TheGeneratedPngs(unittest.TestCase):
    def image(self, locale):
        return Image.open(os.path.join(HERE, f"feature-graphic-{locale}-1024x500.png"))

    def test_rgb_without_alpha_at_1024x500_on_sand(self):
        for locale in ("fr", "en"):
            with self.subTest(locale=locale):
                img = self.image(locale)
                self.assertEqual("RGB", img.mode)
                self.assertEqual((1024, 500), img.size)
                self.assertEqual(SAND, img.getpixel((2, 2)))

    def test_a_filled_pebble_is_cyan_and_a_hollow_one_is_sand_inside(self):
        g = load()
        img = self.image("fr")
        filled = next(p for p in g.PEBBLES if p[4])
        hollow = next(p for p in g.PEBBLES if not p[4])
        centre = lambda p: (int(p[0] + p[2] / 2), int(p[1] + p[3] / 2))
        self.assertEqual(CYAN, img.getpixel(centre(filled)))
        self.assertEqual(SAND, img.getpixel(centre(hollow)))


if __name__ == "__main__":
    unittest.main()
