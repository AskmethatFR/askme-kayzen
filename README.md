# Kayzen

A habit app built on one rule: the gentlest option always wins.

A habit starts at a soft daily goal of 5 minutes. It grows only when *you* decide to
grow it — the system never detects "stability", never suggests a step up, and never
tells you that you failed. A day without a completion is an empty day, not a broken
streak. The board holds at most five habits in parallel, on purpose.

The product decisions behind those sentences are written down, not folklore — see
[`docs/INDEX.md`](docs/INDEX.md).

## Workspace

Two crates, one dependency direction.

| Crate | Contents |
|---|---|
| `core` (`kayzen-core`) | The domain, the use cases, the read-side queries, the in-memory adapters. No UI framework. |
| `app` (`kayzen-app`) | The Dioxus front end: routes, screens, composition root. Depends on `core`; `core` never depends on it. |

Rust 2024 edition, Dioxus 0.7 (`web` by default, `desktop` and `mobile` behind features).

Inside `core`, the layout follows Clean Architecture and DDD: `domain/` holds the
aggregate and its value objects, `use_cases/` the commands, `queries/` the read side
(CQRS-light — commands mutate and return nothing, screens re-query), `infrastructure/`
the adapters. Domain types never cross into `app`; DTOs do.

## Run it

```bash
cargo test --workspace   # every test, both crates
cd app && dx serve       # the web app (cargo install dioxus-cli)
```

## Gates

```bash
scripts/check.sh                          # fmt, clippy, tests, scenario gate, doc anchors
scripts/check.sh new-feature <base-ref>   # the above, plus the mutation gate on the committed diff
```

The scenario gate is bidirectional: every Gherkin scenario in
`docs/functional/features/` resolves to a test through a `// @scenario: <feature>/<Sn>`
anchor, and every anchor resolves back to a scenario. A scenario still tagged `@wip`
is spec-only and waives coverage until its slice lands.

The mutation gate runs on the **committed** diff, so each slice is committed before it
is measured. It blocks on `fix-bug` and `new-feature`, and is advisory on
`quick-change`. Rationale and known blind spots:
[`docs/technical/adr/0009-quality-gates.md`](docs/technical/adr/0009-quality-gates.md).

CI runs `scripts/check.sh` on every PR and on `main`. The PR run additionally computes
the merge-base against `main` and uses it as the mutation gate's base-ref. The gate
implementations are pinned under `scripts/vendor/`, refreshed only by an explicit commit.

## Documentation

`docs/` is a graph, not a folder: [`docs/INDEX.md`](docs/INDEX.md) is the table, and the
`[[links]]` inside each node are its edges. Technical nodes (architecture, ADRs) and
functional nodes (feature catalog, glossary, backlog, Gherkin) are separate lanes.
Start at the index. The functional and design nodes are written in French.

## License

[PolyForm Noncommercial 1.0.0](LICENSE) — free to read, use, modify and share **for any
noncommercial purpose**. Commercial use is not granted. This is a source-available
license, not an OSI-approved open-source one.
