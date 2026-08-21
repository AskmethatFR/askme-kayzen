#!/usr/bin/env python3
"""scenario_audit.py — bidirectional Gherkin scenario <-> test coverage gate.

Downstream project repos carry Gherkin `.feature` files as SPEC-ONLY artefacts:
valid Gherkin, parsed but never executed, zero step definitions, no Cucumber
runner. Each scenario is materialized as a real test elsewhere in the codebase,
carrying a machine-readable anchor:

  - C# (xUnit):  [Trait("Scenario","<feature-id>/<scenario-id>")] on a
                 [Fact]/[Theory] method.
  - Rust:        // @scenario: <feature-id>/<scenario-id>  immediately above a
                 #[test]/#[rstest] fn (Rust has no native trait attribute, so
                 the comment IS the machine anchor).

This module answers, bidirectionally:

  - feature -> test  ("uncovered"): does every non-@wip scenario have >=1 test?
  - test -> feature  ("orphans"):   does every test anchor resolve to a real,
                                     still-existing scenario?

A one-way check is satisfiable by only writing scenarios for what is already
tested; the reverse direction is what catches a scenario silently renamed or
deleted out from under its test. Both directions block; @wip is the single
*reported* waiver, exempting a scenario from "uncovered" only — never from
"orphans" or "malformed" (see AD-4 / C3 in the tech spec).

Two structural checks make this a gate rather than a report:
  - feature-id double check: `# id: x` header must equal the `@feature:x` tag.
    A mismatch is `malformed`, not a warning — copy-paste is how .feature
    files actually get created, and a silent mismatch orphans a whole feature.
  - anchor attachment: the machine anchor must sit directly on a real test
    (Rust: a #[test]/#[rstest] attribute in the contiguous comment/attribute
    block immediately preceding the fn — anything else in between, a `use`,
    `struct`, `impl`, a `mod` boundary, breaks attachment even when a real
    test exists further down the file; C#: the Trait attribute must be in
    the same contiguous attribute block as [Fact]/[Theory], scanned in both
    directions since C# attribute order is arbitrary). Without this the
    tool measures "a comment/attribute exists", not "a test carries the
    scenario".

Pure functions + a thin argparse CLI (house style, see lib/tdd_audit.py).
Stdlib-only. `main()` never raises; the exit code carries the verdict, which
is one of "pass" / "fail" / "empty" (zero scenarios found — see main()'s
docstring and `--allow-empty`).
"""

import argparse
import json
import os
import re
import sys
import unicodedata
from pathlib import Path


def _reraise(exc):
    raise exc


def _bracket_delta(line):
    """Net `[`/`]` balance of one line — used by both the Rust and C#
    attachment scanners to track whether an attribute opened on this line
    (or an earlier one) is still open (SWEEP 1): a `#[should_panic(\n
    expected = "boom"\n)]`-style continuation spans several physical lines,
    and only the bracket balance — not any single line's own shape — says
    whether we are still "inside" that attribute."""
    return line.count("[") - line.count("]")


def _walk_files(root_path, suffix):
    """Like `Path(root_path).rglob(f'*{suffix}')`, sorted, but NEVER follows
    symlinks (retry FOLD IN — Security LOW): `rglob`'s own symlink-cycle
    protection is interpreter-version-gated (Python 3.13+ only), so a
    hostile symlink cycle in a downstream repo could hang the walk on an
    older interpreter. `followlinks=False` never descends into a symlinked
    directory at all, independent of interpreter version. `onerror=_reraise`
    is deliberate too: a directory os.walk cannot list (e.g. permission
    denied) is surfaced as a real exception rather than silently skipped —
    silently skipping would itself be a coverage gap (scenarios in that
    directory going unscanned with no diagnostic), and main()'s own
    try/except is what turns this into a clean {"error": ...} exit 1 rather
    than an uncaught traceback.
    """
    matches = []
    for dirpath, _dirnames, filenames in os.walk(
            str(root_path), onerror=_reraise, followlinks=False):
        for name in filenames:
            if name.endswith(suffix):
                matches.append(Path(dirpath) / name)
    return sorted(matches)


_HEADER_ID_RE = re.compile(r"^#\s*id:\s*(.+)$")
_HEADER_CONTEXT_RE = re.compile(r"^#\s*context:\s*(.+)$")
_HEADER_ORIGIN_RE = re.compile(r"^#\s*origin:\s*(.+)$")
_TAG_TOKEN_RE = re.compile(r"@\S+")
_FEATURE_LINE_RE = re.compile(r"^Feature:\s*(.*)$")
_SCENARIO_LINE_RE = re.compile(r"^(?:Scenario|Scenario Outline):\s*(.*)$")
# fail-LOUD guard (Security MEDIUM probe D fold-in) — the narrow Outline fix
# above only closed `Scenario Outline:` specifically. Any OTHER scenario
# keyword variant (the official Gherkin synonym `Scenario Template:`, or a
# near-miss like `Scenario Outline foo:`) is still invisible to
# `_SCENARIO_LINE_RE` and would otherwise fall through to "free text:
# nothing to record" with zero diagnostic, silently leaking any pending
# `@scenario:` tag onto the next real Scenario: line. `\b` (whole-word
# boundary) after "Scenario" means `ScenarioFoo:` — not attempting the
# keyword at all — does NOT match; `Background:`/`Feature:`/`Examples:`
# never match either, since they don't start with the word "Scenario".
_SCENARIO_KEYWORD_ATTEMPT_RE = re.compile(r"^Scenario\b")


def _strip_leading_invisible_codepoints(line):
    """Strip leading Unicode format-category (Cf) codepoints — zero-width
    space U+200B, BOM U+FEFF, word joiner U+2060, and similar invisible
    control codepoints — that `str.strip()` does NOT remove (Security LOW
    fold-in, probe: `str.strip()` only strips *whitespace*, category Zs/
    control, never Cf). Returns `line` unchanged if it carries no such
    leading codepoint, so callers can cheaply tell "nothing stripped" apart
    from "something was hiding at the start of this line" via `!=`."""
    i = 0
    while i < len(line) and unicodedata.category(line[i]) == "Cf":
        i += 1
    return line[i:]


def parse_feature_file(text, path):
    """Parse one `.feature` file's text into its structural content.

    Returns {"feature_id", "context", "origins", "scenarios", "malformed"} —
    see the module docstring for the shape. Only scenarios carrying an
    `@scenario:<id>` tag participate — an untagged Scenario: has nothing to
    key it by and is not part of the gate. This is a line-based scan of the
    house header convention, not a general Gherkin parser (see cc-kiss:
    building a full AST is exactly the trap a tool like this must resist).
    """
    feature_id = None
    header_id = None
    context = None
    origins = []
    feature_seen = False
    pending_tags = []  # [(tag_str, line_no)] since the last Feature:/Scenario:
    scenarios = []
    malformed = []

    for line_no, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue

        m_id = _HEADER_ID_RE.match(line)
        if m_id:
            header_id = m_id.group(1).strip()
            continue

        m_context = _HEADER_CONTEXT_RE.match(line)
        if m_context:
            context = m_context.group(1).strip()
            continue

        m_origin = _HEADER_ORIGIN_RE.match(line)
        if m_origin:
            origins = [o.strip() for o in m_origin.group(1).split(",") if o.strip()]
            continue

        if line.startswith("#"):
            continue

        if line.startswith("@"):
            for tag in _TAG_TOKEN_RE.findall(line):
                if tag.startswith("@scenario:") and not feature_seen:
                    malformed.append({
                        "file": path, "line": line_no,
                        "problem": f"scenario-tag-outside-feature: {tag}",
                    })
                pending_tags.append((tag, line_no))
            continue

        m_feature = _FEATURE_LINE_RE.match(line)
        if m_feature:
            feature_seen = True
            feat_entry = next(
                (t for t in pending_tags if t[0].startswith("@feature:")), None)
            if feat_entry is None:
                # retry BLOCKING 3 — a missing @feature: tag used to leave
                # feature_id=None with NO malformed entry, silently dropping
                # every scenario in the file (feature -> test direction
                # disabled with zero diagnostic). Surface it instead.
                malformed.append({
                    "file": path, "line": line_no,
                    "problem": "missing-feature-tag: no @feature: tag found for this Feature:",
                })
            else:
                feature_id = feat_entry[0][len("@feature:"):]
                if header_id is None:
                    # retry BLOCKING 3 (related) — a missing # id: header
                    # made the double-check vacuous: with nothing to compare
                    # against, an arbitrarily typo'd @feature: tag was never
                    # flagged. One principle: the identity declaration must
                    # be present AND self-consistent on both sides.
                    malformed.append({
                        "file": path, "line": feat_entry[1],
                        "problem": (f"missing-id-header: no '# id:' header "
                                    f"for '@feature:{feature_id}'"),
                    })
                elif header_id != feature_id:
                    malformed.append({
                        "file": path, "line": feat_entry[1],
                        "problem": (f"feature-id-mismatch: '# id: {header_id}' "
                                    f"vs '@feature:{feature_id}'"),
                    })
            pending_tags = []
            continue

        m_scenario = _SCENARIO_LINE_RE.match(line)
        if m_scenario:
            scenario_tag = next(
                (t for t in pending_tags if t[0].startswith("@scenario:")), None)
            if scenario_tag:
                scenarios.append({
                    "id": scenario_tag[0][len("@scenario:"):],
                    "name": m_scenario.group(1).strip(),
                    "tags": [t[0] for t in pending_tags],
                    "line": line_no,
                })
            pending_tags = []
            continue

        if _SCENARIO_KEYWORD_ATTEMPT_RE.match(line):
            # An unrecognized "Scenario"-shaped keyword (a Gherkin synonym
            # like `Scenario Template:`, or a near-miss like `Scenario
            # Outline foo:`) — surface it loudly instead of silently
            # swallowing it, and clear pending_tags exactly like the
            # recognized branch above so no tag leaks onto the next real
            # scenario.
            malformed.append({
                "file": path, "line": line_no,
                "problem": f"unrecognized-scenario-keyword: {line}",
            })
            pending_tags = []
            continue

        unmasked = _strip_leading_invisible_codepoints(line)
        if unmasked != line and _SCENARIO_KEYWORD_ATTEMPT_RE.match(unmasked):
            # Security LOW fold-in — a leading zero-width/BOM/word-joiner
            # codepoint hides a "Scenario"-shaped line from BOTH checks
            # above (they never even see it: `line` itself does not start
            # with the literal letter "S"). Deliberately narrow: only a
            # line that IS a scenario-keyword attempt once the invisible
            # prefix is removed triggers this — an invisible char on an
            # ordinary step or a table cell is untouched (`unmasked` would
            # not match `_SCENARIO_KEYWORD_ATTEMPT_RE`, or nothing would be
            # stripped at all since the invisible codepoint isn't leading).
            # Deliberately NOT normalized-and-recognized even when
            # `unmasked` would otherwise be the exact recognized form
            # (`Scenario:`/`Scenario Outline:`) — silently registering it
            # would make this tool count a scenario invisible to any
            # reference Gherkin tooling reading the same bytes; a loud
            # diagnostic is the only safe answer either way.
            malformed.append({
                "file": path, "line": line_no,
                "problem": f"invisible-codepoint: {line!r}",
            })
            pending_tags = []
            continue

        # A step (Given/When/Then/And/But) or free text: nothing to record.

    return {
        "feature_id": feature_id, "context": context, "origins": origins,
        "scenarios": scenarios, "malformed": malformed,
    }


def scan_features(features_root):
    """Walk `features_root` for `*.feature` files and index every tagged
    scenario by its `<feature-id>/<scenario-id>` ref.

    Returns {"refs": {ref: {"feature_file", "line", "name", "tags"}},
    "malformed": [...]}. A missing/empty directory is not an error — it
    returns empty collections (a downstream repo may have zero features yet,
    e.g. mid-cycle on a multi-slice ticket). A file whose `@feature:` tag
    mismatches its `# id:` header is still indexed under the (wrong) tag —
    the malformed entry is what surfaces the copy-paste problem, dropping
    the scenario silently would hide it instead.
    """
    root = Path(features_root)
    if not root.is_dir():
        return {"refs": {}, "malformed": []}
    return _index_feature_paths(_walk_files(root, ".feature"))


# Directories never worth walking for authored `.feature` specs — vendored
# deps, build output, VCS/tooling metadata. Pruned during auto-discovery
# (`discover_feature_files`) so a vendored/copied `.feature` under one of
# these never pollutes the audit. Not applied to an EXPLICIT --features-root
# (if the caller points at a tree, they mean that tree, whole).
_DISCOVERY_PRUNE_DIRS = frozenset({
    ".git", ".hg", ".svn", "node_modules", "target", "bin", "obj", "dist",
    "build", "out", ".venv", "venv", "__pycache__", ".next", ".nuxt",
    ".idea", ".vscode", ".tox", ".mypy_cache", ".pytest_cache", "vendor",
})


def discover_feature_files(root):
    """Walk `root` for `*.feature`, pruning `_DISCOVERY_PRUNE_DIRS`, never
    following symlinks. Returns a sorted list of paths. Used when no explicit
    `--features-root` is given, so the tool finds specs wherever a downstream
    repo keeps them (`doc/specs/features`, `features/`, …) instead of relying
    on a single hard-coded default that a differently-laid-out repo silently
    misses. `onerror=_reraise` (as in `_walk_files`) so a permission-denied
    directory surfaces as a real error, caught by main()'s try/except."""
    matches = []
    for dirpath, dirnames, filenames in os.walk(
            str(root), onerror=_reraise, followlinks=False):
        dirnames[:] = [d for d in dirnames if d not in _DISCOVERY_PRUNE_DIRS]
        for name in filenames:
            if name.endswith(".feature"):
                matches.append(Path(dirpath) / name)
    return sorted(matches)


_DISCOVERY_TEST_DIR_NAMES = frozenset({"src", "tests"})


def discover_test_roots(root):
    """Walk `root` for directories named `src` / `tests`, pruning
    `_DISCOVERY_PRUNE_DIRS`, never following symlinks. Returns paths RELATIVE to
    `root` (sorted), so they slot straight into `--tests-root`'s contract. A
    matched directory is not descended into, so a nested `src/tests` never
    yields the same anchors twice.

    Mirrors `discover_feature_files` on the tests side, and exists because the
    two sides were NOT symmetric: an omitted `--features-root` auto-discovers,
    while an omitted `--tests-root` fell back to the hard-coded `src`, `tests`
    and nothing else. In a multi-crate Cargo workspace (or a multi-project .NET
    solution) the code lives in `<member>/src`, so both defaults match nothing,
    `scan_tests` sees zero anchors, and EVERY scenario is reported `uncovered` —
    a false `fail` over a whole repo, not a real coverage gap. Used only as a
    fallback when `--tests-root` is omitted AND neither default exists, so a
    layout that already resolves keeps its exact prior behaviour.
    """
    found = []
    for dirpath, dirnames, _ in os.walk(
            str(root), onerror=_reraise, followlinks=False):
        dirnames[:] = [d for d in dirnames if d not in _DISCOVERY_PRUNE_DIRS]
        for name in list(dirnames):
            if name in _DISCOVERY_TEST_DIR_NAMES:
                found.append((Path(dirpath) / name).relative_to(root))
                dirnames.remove(name)
    return sorted(str(p) for p in found)


def _index_feature_paths(feature_paths):
    """Index every `@scenario:`-tagged scenario across `feature_paths` by its
    `<feature-id>/<scenario-id>` ref (see `scan_features` for the returned
    shape and the malformed-surfacing rules)."""
    refs = {}
    malformed = []
    for feature_path in feature_paths:
        try:
            text = feature_path.read_text(encoding="utf-8")
        except OSError:
            continue
        parsed = parse_feature_file(text, str(feature_path))
        malformed.extend(parsed["malformed"])
        if not parsed["feature_id"]:
            continue
        for scenario in parsed["scenarios"]:
            ref = f"{parsed['feature_id']}/{scenario['id']}"
            if ref in refs:
                # FOLD IN — the same ref defined twice used to silently
                # overwrite with no diagnostic. Keep the first occurrence,
                # flag the duplicate: same principle as the missing-tag fix
                # (BLOCKING 3) — a spec error surfaces loudly, never mutely.
                malformed.append({
                    "file": str(feature_path), "line": scenario["line"],
                    "problem": (f"duplicate-scenario-ref: {ref} already "
                                f"defined in {refs[ref]['feature_file']}"),
                })
                continue
            refs[ref] = {
                "feature_file": str(feature_path),
                "line": scenario["line"],
                "name": scenario["name"],
                "tags": scenario["tags"],
            }
    return {"refs": refs, "malformed": malformed}


_CSHARP_TRAIT_RE = re.compile(r'Trait\("Scenario",\s*"([^"]+)"\)')
_CSHARP_TEST_ATTR_RE = re.compile(r"\[\s*(Fact|Theory)\b")
_CSHARP_BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/")
_CSHARP_LINE_COMMENT_RE = re.compile(r"//.*$")


def _strip_csharp_comments(line):
    """Remove `/* ... */` and trailing `// ...` content from one line —
    SWEEP 5 nit: without this, a decoy like `[/* [Fact] */]` above an
    ordinary method matched `_CSHARP_TEST_ATTR_RE` on the raw text and gave
    a false ACCEPT (worse than a false reject — the whole point of C7)."""
    return _CSHARP_LINE_COMMENT_RE.sub("", _CSHARP_BLOCK_COMMENT_RE.sub("", line))


def _csharp_line_depths(lines):
    """Depth-AT-START of each line, tracking cumulative `[`/`]` balance
    top-to-bottom over the whole file (SWEEP 1): depth > 0 means the line is
    inside an unterminated (multi-line) attribute continuation, e.g. the
    `1, 2)]` line of a `[InlineData(\n 1, 2)]` split across two lines. Never
    goes negative — stray/unbalanced content elsewhere in the file must not
    poison later attribute blocks."""
    depths = []
    depth = 0
    for line in lines:
        depths.append(depth)
        depth = max(0, depth + _bracket_delta(line))
    return depths


def _csharp_attribute_block_bounds(lines, idx, depths):
    """Return (start, end) 0-based inclusive bounds of the maximal
    contiguous run of attribute lines (single- or multi-line) containing
    `idx` — retry BLOCKING 1(b): C# attribute order is arbitrary, so the
    block is walked in BOTH directions, not forward-only from the Trait
    line. SWEEP 1: a line belongs to the block if it opens a fresh
    attribute (`startswith("[")`) OR its start-depth is already > 0 (it is
    a continuation of an attribute opened on an earlier line)."""
    def in_block(i):
        return depths[i] > 0 or lines[i].strip().startswith("[")

    start = idx
    while start - 1 >= 0 and in_block(start - 1):
        start -= 1
    end = idx
    while end + 1 < len(lines) and in_block(end + 1):
        end += 1
    return start, end


def _csharp_anchor_is_attached(lines, trait_idx):
    """True if a [Fact]/[Theory] attribute sits anywhere in the same
    contiguous attribute block as the Trait line at `trait_idx` (0-based),
    scanned in both directions (see `_csharp_attribute_block_bounds`)."""
    depths = _csharp_line_depths(lines)
    start, end = _csharp_attribute_block_bounds(lines, trait_idx, depths)
    return any(_CSHARP_TEST_ATTR_RE.search(_strip_csharp_comments(lines[k]))
               for k in range(start, end + 1))


def extract_csharp_anchors(text):
    """Find every `Trait("Scenario","<ref>")` anchor in C# source (retry
    BLOCKING 1(c): matched without requiring a `[` immediately before
    `Trait` — `[Fact, Trait("Scenario","x")]` is legal C# combining two
    attributes in one bracket, and the old anchored regex made that anchor
    entirely invisible: zero anchors extracted, not even ok=False, so the
    scenario silently reported `uncovered` with no diagnostic pointing at
    the anchor at all).

    Returns [{"ref", "line", "ok"}] — `ok` is False when the Trait is not
    attached to a [Fact]/[Theory] method (AC-T2.3: an attribute that exists
    but carries no real test is malformed, not coverage).
    """
    lines = text.splitlines()
    anchors = []
    for idx, line in enumerate(lines):
        m = _CSHARP_TRAIT_RE.search(line)
        if not m:
            continue
        anchors.append({
            "ref": m.group(1), "line": idx + 1,
            "ok": _csharp_anchor_is_attached(lines, idx),
        })
    return anchors


_RUST_SCENARIO_RE = re.compile(r"//\s*@scenario:\s*(\S+)")
_RUST_TEST_ATTR_RE = re.compile(r"^\s*#\[\s*(?:\w+::)?(test|rstest)\b")
_RUST_ATTR_LINE_RE = re.compile(r"^\s*#\[")
_RUST_COMMENT_LINE_RE = re.compile(r"^\s*//")
_RUST_FN_RE = re.compile(r"\bfn\s")


def _rust_anchor_is_attached(lines, comment_idx):
    """True if a #[test]/#[rstest] attribute appears in the CONTIGUOUS
    comment/attribute block immediately preceding the next `fn` — retry
    BLOCKING 1(a): anything else in between (a `use`, `struct`, `impl`, a
    `mod` boundary, ...) breaks attachment immediately, even when a real
    #[test] fn exists further down the file. The old walk silently skipped
    any non-fn/non-attribute line, which let an anchor sitting on a `use`
    reach an unrelated #[test] several lines away.

    SWEEP 1: a `#[should_panic(\n expected = "boom"\n)]`-shaped attribute
    spans multiple physical lines — its continuation lines (e.g.
    `expected = "boom"`) match none of blank/test-attr/attribute-start/
    comment/fn, so `depth` (net `[`/`]` balance) tracks whether we are still
    inside such an attribute and, while so, every line is consumed without
    re-classification: it belongs to the attribute, not to "unrelated code".
    """
    j = comment_idx + 1
    saw_test_attr = False
    depth = 0
    while j < len(lines):
        stripped = lines[j].strip()
        if depth > 0:
            depth = max(0, depth + _bracket_delta(stripped))
            j += 1
            continue
        if not stripped:
            j += 1
            continue
        if _RUST_TEST_ATTR_RE.search(stripped):
            saw_test_attr = True
            delta = _bracket_delta(stripped)
            if delta > 0:
                depth = delta
            elif _RUST_FN_RE.search(stripped):
                # SWEEP 5 nit — `#[test] fn t() {}` on ONE line: without this,
                # the branch above continued past the fn on this same line,
                # never observing it, and only found the correct answer by
                # accident if a later line happened to be a bare `fn`.
                return True
            j += 1
            continue
        if _RUST_ATTR_LINE_RE.match(stripped) or _RUST_COMMENT_LINE_RE.match(stripped):
            delta = _bracket_delta(stripped)
            if delta > 0:
                depth = delta
            j += 1
            continue
        if _RUST_FN_RE.search(stripped):
            return saw_test_attr
        return False
    return False


def extract_rust_anchors(text):
    """Find every `// @scenario: <ref>` anchor in Rust source.

    Returns [{"ref", "line", "ok"}] — `ok` is False unless a #[test]/#[rstest]
    attribute is found in the CONTIGUOUS comment/attribute block immediately
    preceding the next `fn` (see `_rust_anchor_is_attached`): a comment on a
    helper function is not coverage, and neither is a comment sitting above
    unrelated code (a `use`, `struct`, `impl`, a `mod` boundary) that merely
    happens to precede a real test further down the file.
    """
    lines = text.splitlines()
    anchors = []
    for idx, line in enumerate(lines):
        m = _RUST_SCENARIO_RE.search(line)
        if not m:
            continue
        anchors.append({
            "ref": m.group(1), "line": idx + 1,
            "ok": _rust_anchor_is_attached(lines, idx),
        })
    return anchors


def scan_tests(roots):
    """Walk every directory in `roots` for `*.cs` / `*.rs` files and collect
    their scenario anchors.

    Returns [{"ref", "file", "line", "ok"}], sorted by file path for
    deterministic output. A missing root is skipped, not an error.
    """
    anchors = []
    for root in roots:
        root_path = Path(root)
        if not root_path.is_dir():
            continue
        for cs_path in _walk_files(root_path, ".cs"):
            try:
                text = cs_path.read_text(encoding="utf-8")
            except OSError:
                continue
            for a in extract_csharp_anchors(text):
                anchors.append({"ref": a["ref"], "file": str(cs_path),
                                 "line": a["line"], "ok": a["ok"]})
        for rs_path in _walk_files(root_path, ".rs"):
            try:
                text = rs_path.read_text(encoding="utf-8")
            except OSError:
                continue
            for a in extract_rust_anchors(text):
                anchors.append({"ref": a["ref"], "file": str(rs_path),
                                 "line": a["line"], "ok": a["ok"]})
    return anchors


def audit(feature_refs, test_anchors):
    """Reduce scanned features + test anchors to the bidirectional report
    (AD-4): feature -> test ("uncovered") AND test -> feature ("orphans").
    @wip exempts a scenario from "uncovered" ONLY — never from "orphans" or
    "malformed" (C3 / AC-T2.2).

    `feature_refs`: {ref: {"feature_file", "line", "name", "tags"}} — from
      scan_features()["refs"].
    `test_anchors`: [{"ref", "file", "line", "ok"}] — from scan_tests().

    Returns {"scenarios", "covered", "uncovered", "orphans", "waived",
    "malformed"} — NO "verdict" key (retry BLOCKING 5): the verdict is a
    SINGLE truth owned by main() alone, computed once from the FULL merged
    malformed list. `malformed` here covers ONLY anchor-attachment failures
    (ok=False entries in `test_anchors`) — structural feature-file malformed
    (id mismatch, missing tag/header, tag-outside-feature) lives in
    scan_features()'s own "malformed" list; audit()'s fixed two-argument
    signature carries no raw feature-file text to re-derive it from, so the
    CLI (main()) merges the two lists before deciding pass/fail/empty. A
    library caller computing ITS OWN verdict from audit()'s output alone
    must also account for scan_features()'s malformed list, exactly as
    main() does.
    """
    valid = [a for a in test_anchors if a.get("ok")]
    anchor_malformed = [
        {"file": a["file"], "line": a["line"],
         "problem": f"anchor-not-attached-to-test: {a['ref']}"}
        for a in test_anchors if not a.get("ok")
    ]

    tests_by_ref = {}
    for a in valid:
        tests_by_ref.setdefault(a["ref"], []).append(f"{a['file']}:{a['line']}")

    covered, uncovered, waived = [], [], []
    for ref in sorted(feature_refs):
        info = feature_refs[ref]
        if ref in tests_by_ref:
            covered.append({"ref": ref, "tests": tests_by_ref[ref]})
        elif "@wip" in info.get("tags", []):
            waived.append({"ref": ref, "reason": "@wip"})
        else:
            uncovered.append({
                "ref": ref, "feature_file": info.get("feature_file"),
                "line": info.get("line"),
            })

    orphans = [
        {"ref": a["ref"], "file": f"{a['file']}:{a['line']}"}
        for a in valid if a["ref"] not in feature_refs
    ]

    return {
        "scenarios": len(feature_refs),
        "covered": covered,
        "uncovered": uncovered,
        "orphans": orphans,
        "waived": waived,
        "malformed": anchor_malformed,
    }


def _report_discovery(root, discovered):
    """Emit a one-line stderr breadcrumb naming what auto-discovery found (or
    that it found nothing) — the only signal a caller gets that the canonical
    default path was not used. Never touches stdout (the machine-readable
    report / verdict)."""
    if discovered:
        dirs = sorted({
            str(p.parent.relative_to(root)) for p in discovered
        })
        shown = ", ".join(dirs[:5]) + ("…" if len(dirs) > 5 else "")
        print(
            f"scenario-audit: auto-discovered {len(discovered)} .feature "
            f"file(s) under {root} (dirs: {shown}); pass --features-root to "
            f"pin an explicit tree",
            file=sys.stderr,
        )
    else:
        print(
            f"scenario-audit: auto-discovery found no .feature files under "
            f"{root}; pass --features-root if specs live in an unusual location",
            file=sys.stderr,
        )


def _emit_misconfigured(args, tests_roots, root, features_root_label):
    """An EXPLICIT `--features-root` that does not exist is a config error,
    not a coverage failure. Emit a distinct `misconfigured` verdict + a loud
    stderr hint (naming any tree auto-discovery WOULD have found), then exit 1.
    Never suppressed by --allow-empty (that hatch is only for a genuine
    zero-scenario `empty`). Called from inside main()'s try/except, so a walk
    error while probing for the suggestion still degrades to the clean
    {"error": ...} exit rather than a traceback."""
    suggestion = ""
    try:
        discovered = discover_feature_files(root)
    except Exception:
        discovered = []
    if discovered:
        dirs = sorted({str(p.parent.relative_to(root)) for p in discovered})
        suggestion = f" Did you mean: {', '.join(dirs[:5])}?"
    anchor_count = 0
    try:
        anchor_count = len(scan_tests([str(root / t) for t in tests_roots]))
    except Exception:
        pass
    print(
        f"scenario-audit: MISCONFIGURED --features-root "
        f"'{features_root_label}' does not exist under {root} "
        f"(found {anchor_count} test anchor(s), 0 scenarios).{suggestion}",
        file=sys.stderr,
    )
    report = {
        "schema": "scenario-audit/v1",
        "features_root": features_root_label,
        "tests_roots": tests_roots,
        "allow_empty": args.allow_empty,
        "scenarios": 0,
        "covered": [],
        "uncovered": [],
        "orphans": [],
        "waived": [],
        "malformed": [],
        "verdict": "misconfigured",
    }
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print("scenario-audit: misconfigured (--features-root does not exist)")
    sys.exit(1)


def main():
    """Thin argparse CLI. Never raises; the exit code carries the verdict.

    `--root` is mandatory and never inferred from cwd (AC-T2.4): the tool
    lives in this config repo but always executes against a downstream
    project repo, and an explicit --root enforces that separation
    structurally. `--features-root` / `--tests-root` are resolved relative
    to --root so the placement decision stays a flag change, never a
    rewrite (C5).

    Owns the ONE verdict (retry BLOCKING 5): "fail" whenever uncovered,
    orphans, or ANY malformed (structural or anchor) is non-empty; else
    "empty" whenever scenarios==0 (retry BLOCKING 4 — Security HIGH: a
    --root pointed at the wrong checkout, or a features-root that matches
    nothing AND a tests-root that matches nothing, used to report "pass"
    having verified nothing at all — a distinct non-passing verdict closes
    that hole); else "pass". Note the priority: "orphans" (a typo'd
    --features-root with anchors still present in test code) already fails
    via the normal path and is NEVER relabelled "empty" — only the true
    vacuous case (nothing found on EITHER side) hits this branch.

    `--allow-empty` is the explicit escape hatch for the legitimate case
    (scan_features()'s own docstring: a downstream repo mid-adoption may
    have zero features yet) — SWEEP 2: it widens the EXIT CODE only (0
    instead of 1 for an "empty" verdict), it never rewrites the verdict
    itself. The JSON report always says "empty" when scenarios==0 and
    always echoes `allow_empty`, so a downstream CI that leaves the flag on
    permanently stays auditable from the report alone — the exit code by
    itself cannot tell a genuine pass from a suppressed empty. A warning is
    also printed to stderr whenever the flag actually suppresses a
    non-pass exit.
    """
    parser = argparse.ArgumentParser(
        description="Bidirectional Gherkin scenario <-> test coverage gate.")
    parser.add_argument("--root", required=True,
                         help="downstream project repo to audit (mandatory, "
                              "never inferred from cwd)")
    parser.add_argument("--features-root", default=None,
                         help="path, relative to --root, to the .feature tree; "
                              "when omitted, the tool auto-discovers .feature "
                              "files by walking --root (noise dirs pruned)")
    parser.add_argument("--tests-root", action="append", dest="tests_roots",
                         default=None,
                         help="path, relative to --root, to scan for test "
                              "anchors; repeatable (default: src, tests)")
    parser.add_argument("--allow-empty", action="store_true",
                         help="exit 0 on a zero-scenario \"empty\" verdict "
                              "(e.g. a downstream repo mid-adoption) instead "
                              "of the default exit 1 — the report still "
                              "says \"empty\", only the exit code changes")
    parser.add_argument("--json", action="store_true",
                         help="print the machine-readable report")
    args = parser.parse_args()

    tests_roots = args.tests_roots or ["src", "tests"]
    root = Path(args.root)

    # `--features-root` handling (D1 field report):
    #   * omitted  -> AUTO-DISCOVER by walking --root (noise dirs pruned), so
    #     a repo that keeps specs off the canonical path (e.g. doc/specs/
    #     features) is found instead of silently missed.
    #   * explicit but the path does NOT exist -> `misconfigured`, a distinct
    #     LOUD verdict, never a confusing orphans-`fail`. A nonexistent
    #     explicit root is a config error (a typo, a stale default), not a
    #     real coverage gap — and --allow-empty must NOT paper over it.
    #   * explicit and present -> scan exactly that tree (prior behaviour).
    try:
        if args.features_root is None:
            discovered = discover_feature_files(root)
            scanned = _index_feature_paths(discovered)
            features_root_label = "(auto-discovered)"
            _report_discovery(root, discovered)
        else:
            features_path = root / args.features_root
            features_root_label = args.features_root
            if not features_path.exists():
                _emit_misconfigured(
                    args, tests_roots, root, features_root_label)
            scanned = scan_features(str(features_path))
        # An omitted `--tests-root` whose defaults match nothing scans nothing
        # and reports every scenario `uncovered` — a false `fail`. Fall back to
        # discovery (see `discover_test_roots`), and echo what was actually
        # scanned via the report's own `tests_roots` field so the substitution
        # stays auditable rather than silent.
        if args.tests_roots is None and not any(
                (root / t).is_dir() for t in tests_roots):
            discovered_roots = discover_test_roots(root)
            if discovered_roots:
                tests_roots = discovered_roots
        test_anchors = scan_tests([str(root / t) for t in tests_roots])
        result = audit(scanned["refs"], test_anchors)
    except Exception as exc:  # never raise — the exit code carries the verdict
        print(json.dumps({"schema": "scenario-audit/v1", "error": str(exc)}))
        sys.exit(1)

    malformed = list(scanned["malformed"]) + list(result["malformed"])
    if result["uncovered"] or result["orphans"] or malformed:
        verdict = "fail"
    elif result["scenarios"] == 0:
        verdict = "empty"
    else:
        verdict = "pass"

    report = {
        "schema": "scenario-audit/v1",
        "features_root": features_root_label,
        "tests_roots": tests_roots,
        "allow_empty": args.allow_empty,
        "scenarios": result["scenarios"],
        "covered": result["covered"],
        "uncovered": result["uncovered"],
        "orphans": result["orphans"],
        "waived": result["waived"],
        "malformed": malformed,
        "verdict": verdict,
    }

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print(f"scenario-audit: {verdict} ({report['scenarios']} scenarios, "
              f"{len(report['uncovered'])} uncovered, "
              f"{len(report['orphans'])} orphans, "
              f"{len(malformed)} malformed, "
              f"{len(report['waived'])} waived)")

    # SWEEP 2 — verdict and exit code are DECOUPLED: the JSON always states
    # the true verdict ("empty" stays "empty"), so a downstream CI that
    # leaves --allow-empty on permanently remains auditable from the report
    # alone. --allow-empty only widens which verdicts exit 0; it never
    # rewrites what the report says happened. The stderr warning is for
    # whoever is watching logs, not parsing JSON.
    passes = verdict == "pass" or (verdict == "empty" and args.allow_empty)
    if verdict == "empty" and args.allow_empty:
        print("scenario-audit: warning: --allow-empty suppressed a "
              "non-pass (\"empty\") exit — 0 scenarios found", file=sys.stderr)
    sys.exit(0 if passes else 1)


if __name__ == "__main__":
    main()
