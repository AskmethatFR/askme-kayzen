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
                f.write("<html><head></head><body></body></html>")
            root = bs.stage("fr", viewport_h)
            return open(os.path.join(root, "index.html"), encoding="utf-8").read()

    def test_ritual_and_anchored_get_the_phone_height(self):
        html = self.staged_index(640)
        self.assertIn("html .screen.ritual,html .screen.anchored{min-height:640px}", html)

    def test_height_follows_the_viewport_asked_for(self):
        html = self.staged_index(800)
        self.assertIn("min-height:800px", html)


if __name__ == "__main__":
    unittest.main()
