import importlib.util
import os
import re
import tempfile
import unittest

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SAND = (244, 241, 234)
INK = (42, 38, 34)
MUTED = (94, 87, 79)


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


def luminance(rgb):
    lin = [c / 255 / 12.92 if c / 255 <= 0.03928 else ((c / 255 + 0.055) / 1.055) ** 2.4 for c in rgb]
    return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]


def contrast(a, b):
    hi, lo = sorted((luminance(a), luminance(b)), reverse=True)
    return (hi + 0.05) / (lo + 0.05)


APPROVED_NAME_CAPTURE_HEADLINE_SUBLINE = {
    "fr": [
        ("01-aujourdhui", "01-aujourdhui", "Un petit pas suffit.", "Pas de série, pas de pression."),
        ("02-detail", "02-detail", "Un jour sans ? Rien ne casse.", "Aucune série à protéger."),
        ("03-semaine", "04-semaine", "Votre rythme, pas le nôtre.", "Plus ou moins : vous décidez."),
        ("04-rituel", "03-rituel", "Un moment pour vous.", "Arrêter n'est pas échouer."),
        ("05-ancrees", "05-ancrees", "Devenue naturelle ? Ancrez-la.", "Elle libère une place."),
        ("06-ajouter", "06-ajouter", "Commencez tout petit.", "Cinq habitudes au plus."),
    ],
    "en": [
        ("01-aujourdhui", "01-aujourdhui", "One small step is enough.", "No streaks, no pressure."),
        ("02-detail", "02-detail", "Skip a day. Nothing breaks.", "No streak to protect."),
        ("03-semaine", "04-semaine", "Your pace, not ours.", "More or less: you decide."),
        ("04-rituel", "03-rituel", "A moment for yourself.", "Stopping isn't failing."),
        ("05-ancrees", "05-ancrees", "Became natural? Anchor it.", "It frees up a seat."),
        ("06-ajouter", "06-ajouter", "Start tiny.", "Five habits at most."),
    ],
}


class TilesTable(unittest.TestCase):
    def test_six_tiles_per_locale_each_with_a_capture_and_an_int_crop_top(self):
        t = load()
        for locale in ("fr", "en"):
            self.assertEqual(6, len(t.TILES[locale]))
            for tile in t.TILES[locale]:
                self.assertIsInstance(tile.crop_top, int, tile.name)
                self.assertTrue(os.path.exists(t.capture_path(locale, tile.capture)), tile.name)

    def test_the_table_is_the_approved_copy_in_the_approved_order(self):
        t = load()
        for locale in ("fr", "en"):
            with self.subTest(locale=locale):
                self.assertEqual(
                    APPROVED_NAME_CAPTURE_HEADLINE_SUBLINE[locale],
                    [(x.name, x.capture, x.headline, x.subline) for x in t.TILES[locale]],
                )

    def test_tiles_02_and_05_put_the_caption_at_the_bottom_and_the_others_on_top(self):
        t = load()
        for locale in ("fr", "en"):
            layouts = [x.layout for x in t.TILES[locale]]
            self.assertEqual(["top", "bottom", "top", "top", "bottom", "top"], layouts, locale)


class CalloutCrops(unittest.TestCase):
    def test_a_crop_inside_the_capture_passes(self):
        t = load()
        t.check_crop((0, 0, 1080, 1920), (1080, 1920))
        t.check_crop((100, 200, 101, 201), (1080, 1920))

    def test_a_crop_leaving_the_capture_or_empty_raises(self):
        t = load()
        for crop in ((-1, 0, 100, 100), (0, -1, 100, 100), (0, 0, 1081, 100), (0, 0, 100, 1921),
                     (100, 0, 100, 100), (0, 100, 100, 100), (200, 0, 100, 100)):
            with self.subTest(crop=crop), self.assertRaises(ValueError):
                t.check_crop(crop, (1080, 1920))

    def test_every_real_crop_lies_inside_its_own_capture(self):
        t = load()
        for locale in ("fr", "en"):
            for tile in t.TILES[locale]:
                with self.subTest(locale=locale, tile=tile.name):
                    size = Image.open(t.capture_path(locale, tile.capture)).size
                    t.check_crop(tile.crop, size)

    def test_every_zoom_is_between_1_4_and_1_8(self):
        t = load()
        for locale in ("fr", "en"):
            for tile in t.TILES[locale]:
                with self.subTest(locale=locale, tile=tile.name):
                    self.assertTrue(1.4 <= tile.zoom <= 1.8, tile.zoom)

    def test_every_callout_sits_on_the_tile_clear_of_the_caption_and_pops_out_of_the_phone(self):
        t = load()
        right = t.PHONE_LEFT + t.PHONE_W
        for locale in ("fr", "en"):
            for tile in t.TILES[locale]:
                with self.subTest(locale=locale, tile=tile.name):
                    x0, y0, x1, y1 = t.callout_rect(tile)
                    self.assertTrue(0 <= x0 and x1 <= t.TILE_W and 0 <= y0 and y1 <= t.TILE_H)
                    if tile.layout == "top":
                        self.assertGreaterEqual(y0, t.PHONE_TOP)
                    else:
                        self.assertLessEqual(y1, t.TILE_H - t.PHONE_TOP)
                    self.assertTrue(x0 < t.PHONE_LEFT < x1 or x0 < right < x1, (x0, x1))


class CalloutsHideTheirSource(unittest.TestCase):
    def test_every_callout_covers_the_part_of_the_screen_it_was_cropped_from(self):
        t = load()
        for locale in ("fr", "en"):
            for tile in t.TILES[locale]:
                with self.subTest(locale=locale, tile=tile.name):
                    sx0, sy0, sx1, sy1 = t.source_rect(tile)
                    cx0, cy0, cx1, cy1 = t.callout_rect(tile)
                    self.assertTrue(cx0 <= sx0 and cy0 <= sy0 and sx1 <= cx1 and sy1 <= cy1,
                                    ((sx0, sy0, sx1, sy1), (cx0, cy0, cx1, cy1)))

    def test_the_source_of_a_top_tile_sits_on_the_screen_below_the_bezel(self):
        t = load()
        tile = t.Tile("x", "c", 0, "H", "s", "top", (100, 200, 300, 400), 1.5)
        x0, y0, x1, y1 = t.source_rect(tile)
        self.assertEqual(t.PHONE_LEFT + t.FRAME + round(100 * t.SCALE), x0)
        self.assertEqual(t.PHONE_TOP + t.FRAME + round(200 * t.SCALE), y0)
        self.assertEqual((round(200 * t.SCALE), round(200 * t.SCALE)), (x1 - x0, y1 - y0))

    def test_the_source_of_a_bottom_tile_sits_at_the_top_edge_shifted_up_by_its_crop_top(self):
        t = load()
        tile = t.Tile("x", "c", 30, "H", "s", "bottom", (100, 200, 300, 400), 1.5)
        x0, y0, x1, y1 = t.source_rect(tile)
        self.assertEqual(t.PHONE_LEFT + t.FRAME + round(100 * t.SCALE), x0)
        self.assertEqual(round((200 - 30) * t.SCALE), y0)
        self.assertEqual((round(200 * t.SCALE), round(200 * t.SCALE)), (x1 - x0, y1 - y0))

    def test_a_callout_near_the_tile_edge_stays_centred_on_its_source(self):
        t = load()
        tile = t.Tile("x", "c", 0, "H", "s", "top", (0, 0, 1080, 200), 1.4)
        sx0, sy0, sx1, sy1 = t.source_rect(tile)
        cx0, cy0, cx1, cy1 = t.callout_rect(tile)
        self.assertAlmostEqual((sx0 + sx1) / 2, (cx0 + cx1) / 2, delta=1)
        self.assertAlmostEqual((sy0 + sy1) / 2, (cy0 + cy1) / 2, delta=1)

    def test_the_02_callout_top_edge_shows_the_card_background_not_the_divider(self):
        t = load()
        for locale in ("fr", "en"):
            tile = next(x for x in t.TILES[locale] if x.name == "02-detail")
            with self.subTest(locale=locale):
                x0, y0, x1, _ = tile.crop
                gray = Image.open(t.capture_path(locale, tile.capture)).convert("L")
                for y in range(y0, y0 + 10):
                    margin = gray.getpixel((50, y))
                    worst = max(abs(gray.getpixel((x, y)) - margin) for x in range(x0, x1))
                    self.assertLessEqual(worst, 4, (y, worst))


class ChromeMeasuresLineCounts(unittest.TestCase):
    def test_a_sixteen_word_string_runs_on_at_least_three_lines(self):
        t = load()
        [(lines, _)] = t.measure_headlines(t.store_module().CHROME, [" ".join(["mot"] * 16)])
        self.assertGreaterEqual(lines, 3)

    def test_a_two_line_headline_reports_two_lines_and_the_words_of_its_last_line(self):
        t = load()
        probe = "One small step is enough for you today."
        self.assertEqual([(2, 4)], t.measure_headlines(t.store_module().CHROME, [probe]))


class ScreenBox(unittest.TestCase):
    def test_top_layout_looks_below_the_status_bar_down_to_the_tile_edge(self):
        t = load()
        left, top, right, bottom = t.screen_box("top")
        self.assertEqual((t.PHONE_LEFT + t.FRAME, t.PHONE_LEFT + t.FRAME + t.SCREEN_W), (left, right))
        self.assertGreaterEqual(top, t.PHONE_TOP + t.FRAME + t.STATUS_BAR_H)
        self.assertEqual(t.TILE_H, bottom)

    def test_bottom_layout_stays_above_the_caption(self):
        t = load()
        _, top, _, bottom = t.screen_box("bottom")
        self.assertEqual(0, top)
        self.assertLessEqual(bottom, t.TILE_H - t.PHONE_TOP)


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


class HeadlinesStayOnTwoLinesWithoutAnOrphan(unittest.TestCase):
    def test_one_or_two_lines_pass_when_no_line_is_a_lone_word(self):
        t = load()
        t.check_headline_lines("Start tiny.", 1, 2)
        t.check_headline_lines("One word", 1, 1)
        t.check_headline_lines("Votre rythme, pas le nôtre.", 2, 2)

    def test_three_lines_raise(self):
        t = load()
        with self.assertRaises(ValueError):
            t.check_headline_lines("Too long", 3, 2)

    def test_a_last_line_of_one_word_after_a_break_raises(self):
        t = load()
        with self.assertRaises(ValueError):
            t.check_headline_lines("Votre rythme, pas le nôtre.", 2, 1)

    def test_every_real_headline_measured_in_chrome_is_at_most_two_lines_with_no_orphan(self):
        t = load()
        headlines = [x.headline for locale in ("fr", "en") for x in t.TILES[locale]]
        measured = t.measure_headlines(t.store_module().CHROME, headlines)
        self.assertEqual(len(headlines), len(measured))
        for headline, (lines, last_words) in zip(headlines, measured):
            with self.subTest(headline=headline):
                t.check_headline_lines(headline, lines, last_words)


class TextBandStaysWithinItsLimit(unittest.TestCase):
    def test_text_ending_on_the_last_allowed_row_passes(self):
        t = load()
        t.check_text_band(sand_with_line(t.BAND_MAX - 84, t.BAND_MAX))

    def test_text_reaching_one_row_further_raises(self):
        t = load()
        with self.assertRaises(ValueError):
            t.check_text_band(sand_with_line(t.BAND_MAX - 84, t.BAND_MAX + 1))

    def test_a_faint_grey_is_ink_in_a_capture_but_not_a_phone_shadow_in_the_text_band(self):
        t = load()
        shadow = Image.new("RGB", (1080, 1920), SAND)
        shadow.paste((162, 160, 155), (100, 250, 500, t.BAND_MAX + 10))
        self.assertTrue(t.ink_rows(shadow)[260])
        t.check_text_band(shadow)


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
        last = 1920 - t.VISIBLE_ROWS
        t.check_clean_cuts(ink, last)
        with self.assertRaises(ValueError):
            t.check_clean_cuts(ink, last + 1)

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
            for tile in t.TILES[locale]:
                with self.subTest(locale=locale, tile=tile.name):
                    ink = t.ink_rows(Image.open(t.capture_path(locale, tile.capture)))
                    t.check_clean_cuts(ink, tile.crop_top)


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
            tile = t.Tile("01-x", "c", 0, "Hi", "there", "top", (0, 0, 100, 100), 1.5)
            with self.assertRaises(RuntimeError):
                t.compose("chrome", "fr", tile, tmp)
            self.assertEqual([], os.listdir(os.path.join(tmp, "out", "fr")))


def composed_page(t, locale, tile):
    """The tile's HTML as compose() hands it to Chrome, with Chrome stubbed out."""
    with tempfile.TemporaryDirectory() as tmp:
        os.makedirs(os.path.join(tmp, "out", locale))
        t.OUT = os.path.join(tmp, "out")
        t.run_chrome = lambda chrome, args, raw, label: Image.new(
            "RGB", (t.TILE_W, t.TILE_H), SAND
        ).save(raw)
        t.compose("chrome", locale, tile, tmp)
        with open(os.path.join(tmp, f"{tile.name}-tile.html"), encoding="utf-8") as f:
            return f.read()


class ComposedPage(unittest.TestCase):
    def test_the_shadow_reach_is_the_callout_css_shadow_offset_plus_blur(self):
        t = load()
        page = composed_page(t, "fr", t.TILES["fr"][0])
        dy, blur = re.search(r"\.callout \{.*?box-shadow: 0 (\d+)px (\d+)px", page, re.S).groups()
        self.assertEqual(int(dy) + int(blur), t.SHADOW_REACH)

    def test_the_subline_gets_a_no_break_space_before_a_colon_like_the_headline(self):
        t = load()
        tile = next(x for x in t.TILES["fr"] if x.name == "03-semaine")
        self.assertIn('<div class="subline">Plus ou moins : vous décidez.</div>', composed_page(t, "fr", tile))


def copy_band(t, tile, image):
    upright = image.transpose(Image.FLIP_TOP_BOTTOM) if tile.layout == "bottom" else image
    return upright.crop((0, 0, t.TILE_W, t.PHONE_TOP))


def real_tiles(t):
    for locale in ("fr", "en"):
        for tile in t.TILES[locale]:
            yield locale, tile, Image.open(os.path.join(t.OUT, locale, f"{tile.name}-1080x1920.png"))


class TheRenderedTiles(unittest.TestCase):
    def ink_span(self, t, tile, image):
        rows = [y for y, on in enumerate(t.ink_rows(copy_band(t, tile, image), t.BAND_INK_DEVIATION)) if on]
        return rows[0], rows[-1]

    def test_top_captions_start_110_to_150_rows_below_the_top_edge_and_end_at_one_height(self):
        t = load()
        ends = {}
        for locale, tile, image in real_tiles(t):
            if tile.layout == "top":
                first, last = self.ink_span(t, tile, image)
                self.assertTrue(110 <= first <= 150, (locale, tile.name, first))
                ends[f"{locale}/{tile.name}"] = last
        self.assertLessEqual(max(ends.values()) - min(ends.values()), 16, ends)

    def test_bottom_captions_are_centred_between_the_phone_and_the_bottom_edge(self):
        t = load()
        for locale, tile, image in real_tiles(t):
            if tile.layout == "bottom":
                edge_margin, last = self.ink_span(t, tile, image)
                phone_margin = t.PHONE_TOP - 1 - last
                self.assertLessEqual(abs(edge_margin - phone_margin), 16, (locale, tile.name))

    def test_the_wash_beside_the_phone_is_visibly_not_the_captures_own_sand(self):
        t = load()
        for locale, tile, image in real_tiles(t):
            with self.subTest(locale=locale, tile=tile.name):
                got = image.getpixel((20, 1000))
                self.assertGreaterEqual(sum(abs(a - b) for a, b in zip(got, SAND)), 25, got)

    def test_the_caption_is_readable_on_its_background(self):
        t = load()
        for locale, tile, image in real_tiles(t):
            with self.subTest(locale=locale, tile=tile.name):
                strip = copy_band(t, tile, image).crop((0, 0, 40, t.PHONE_TOP))
                darkest = min(strip.getdata(), key=luminance)
                self.assertGreaterEqual(contrast(MUTED, darkest), 4.5, darkest)


if __name__ == "__main__":
    unittest.main()
