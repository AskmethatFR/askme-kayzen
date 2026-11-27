import importlib.util
import os
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))


def load():
    spec = importlib.util.spec_from_file_location(
        "build_screenshots", os.path.join(HERE, "build_screenshots.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class StagePinsFullHeightScreens(unittest.TestCase):
    def staged_index(self, viewport_h):
        bs = load()
        with tempfile.TemporaryDirectory() as tmp:
            bs.PUBLIC = os.path.join(tmp, "public")
            bs.WORK = os.path.join(tmp, "work")
            os.makedirs(bs.PUBLIC)
            with open(os.path.join(bs.PUBLIC, "index.html"), "w", encoding="utf-8") as f:
                f.write("<html><head></head><body><script>APP_START</script></body></html>")
            root = bs.stage("fr", viewport_h)
            return open(os.path.join(root, "index.html"), encoding="utf-8").read()

    def test_ritual_and_anchored_get_the_phone_height(self):
        html = self.staged_index(640)
        self.assertIn("html .screen.ritual,html .screen.anchored{min-height:640px}", html)

    def test_height_follows_the_viewport_asked_for(self):
        html = self.staged_index(800)
        self.assertIn("min-height:800px", html)

    def test_date_and_rng_shims_run_before_the_app(self):
        html = self.staged_index(640)
        self.assertIn("2026,2,17", html)
        self.assertIn("getRandomValues", html)
        app = html.index("APP_START")
        self.assertLess(html.index("2026,2,17"), app)
        self.assertLess(html.index("getRandomValues"), app)


    def test_transitions_are_switched_off_so_no_capture_catches_one_midway(self):
        html = self.staged_index(640)
        self.assertIn("transition:none", html)


class FixedDay(unittest.TestCase):
    def test_the_board_is_anchored_to_the_fixed_day_not_to_today(self):
        bs = load()
        self.assertEqual(bs.today_as_stored(), bs.FIXED_DAY.toordinal())
        self.assertEqual(bs.FIXED_DAY.isoformat(), "2026-03-17")


class BlankDetect(unittest.TestCase):
    def test_a_uniform_white_capture_raises(self):
        from PIL import Image

        bs = load()
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "white.png")
            Image.new("RGB", (100, 100), "white").save(path)
            with self.assertRaises(RuntimeError):
                bs.assert_not_blank(path)

    def test_a_real_capture_passes(self):
        bs = load()
        bs.assert_not_blank(
            os.path.join(HERE, "screenshots", "fr", "01-aujourdhui-1080x1920.png")
        )


if __name__ == "__main__":
    unittest.main()
