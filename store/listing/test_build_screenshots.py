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


class DemoBoardIsTimesPerDay(unittest.TestCase):
    """The seeded board must read as the version-2 times-per-day model."""

    def setUp(self):
        self.bs = load()
        self.snap = self.bs.snapshot("fr")
        self.today = self.bs.today_as_stored()
        self.habits = {h["id"]: h for h in self.snap["habits"]}

    def attempts(self, habit, day):
        return habit["completions"].count(day)

    def goal_on(self, habit, day):
        goals = [s["goal"] for s in habit["steps"] if s["on"] <= day]
        return goals[-1]

    def of_state(self, state):
        return [h for h in self.snap["habits"] if h["state"] == state]

    def test_the_snapshot_is_version_2_so_the_codec_keeps_the_goals(self):
        self.assertEqual(self.snap["v"], 2)

    def test_lire_une_page_grows_from_3_to_4(self):
        steps = self.habits["lire-une-page"]["steps"]
        self.assertEqual([s["goal"] for s in steps], [3, 4])

    def test_lire_une_page_has_an_empty_day_and_sized_pebbles_in_its_last_seven(self):
        lire = self.habits["lire-une-page"]
        week = [self.today - back for back in range(7)]
        attempts = [self.attempts(lire, d) for d in week]
        self.assertIn(0, attempts)
        sized = {round(a / self.goal_on(lire, d), 2) for a, d in zip(attempts, week) if a}
        self.assertGreaterEqual(len(sized), 3)

    def test_today_reads_2_of_3_with_one_partial_habit(self):
        active = self.of_state("Active")
        self.assertEqual(len(active), 3)
        reached = [
            self.attempts(h, self.today) >= self.goal_on(h, self.today) for h in active
        ]
        self.assertEqual(sum(reached), 2)
        partial = [
            h
            for h in active
            if 0 < self.attempts(h, self.today) < self.goal_on(h, self.today)
        ]
        self.assertEqual(len(partial), 1)

    def test_ancrees_reads_4_of_5_with_anchored_habits_present(self):
        self.assertEqual(len(self.snap["habits"]) - len(self.of_state("Anchored")), 4)
        self.assertTrue(self.of_state("Anchored"))

    def test_no_completion_is_dated_after_the_fixed_day(self):
        for habit in self.snap["habits"]:
            self.assertLessEqual(max(habit["completions"]), self.today)


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
