# GB-203 — split miter-engine into polyglot, inventory and analysis

Evidence gathered for GB-160 ("evidence before any further crate extraction"), 2026-10-31.

## Measured inventory (126 files, ~9.3k lines)

| subsystem | lines | files |
|:--|--:|--:|
| polyglot | 2488 | 17 |
| analysis | 2346 | 16 |
| inventory | 1046 | 10 |
| hydrate | 960 | 7 |
| codec | 742 | 5 |
| security | 599 | 3 |
| store | 406 | 4 |
| ingest | 317 | 3 |
| root files (lib, config, persist) | 376 | 3 |

## Which dependency belongs to which subsystem (files mentioning it)

| dep | polyglot | analysis | inventory | other |
|:--|--:|--:|--:|:--|
| tree_sitter | 6 | 9 | 0 | — |
| sqlparser | 1 | 1 | 0 | — |
| hcl | 4 | 0 | 0 | — |
| dockerfile_parser | 2 | 1 | 0 | — |
| serde_yaml | 4 | 0 | 0 | — |
| globset | 0 | 1 | 0 | — |
| rayon | 0 | 0 | 0 | store=1, ingest=1 |
| redb | 1 | 5 | 0 | codec=1, store=1 |

Three findings that shape the split:

1. **`tree_sitter` spans polyglot (6) and analysis (9).** So `miter-analysis` depends on `miter-polyglot`: a
downward edge, which satisfies the layer rule rather than violating it. The split order therefore has to be
polyglot first, analysis second.
2. **The language parsers are polyglot's alone**: hcl, serde_yaml, dockerfile-parser and sqlparser appear
almost nowhere else. That is the heavy half of the crate by dependency weight, and it is already
concentrated.
3. **`redb` in `analysis` is not a coupling to freeze in**: the hits are store-coupled convenience wrappers
(`verify_contracts_from_store`, `analyze_hygiene(store)`) sitting beside pure cores
(`verify_contracts_from_symbols`). The pure functions are what move to `miter-analysis`; the wrappers stay
behind in the facade, which is what a facade is for.

## Proposed shape

- `miter-polyglot`: language extraction and the universal symbol model. Owns tree_sitter, sqlparser, hcl,
dockerfile-parser, serde_yaml. Leaf crate.
- `miter-analysis`: pure passes over symbols (contracts, cycles, clones, hygiene, impact, deps). Depends on
`miter-polyglot`. No storage dependency once the wrappers stay behind.
- `miter-inventory`: file walking and AST inventory. Parser-free today (no tree_sitter hits), so it can move
without pulling the language stack.
- `miter-engine` stays as the facade: hydrate, codec, store, persist, ingest, security, and the
`*_from_store` entry points, re-exporting the three crates so no caller changes a path.

## Sequencing

1. `miter-polyglot` (leaf, no in-crate edges) — move, re-export from engine, gate.
2. `miter-analysis` (depends on polyglot) — move pure passes, keep wrappers, gate.
3. `miter-inventory` — move, gate.
4. Delete the now-empty modules from engine and confirm the re-exports still satisfy every caller.

## Open risk

`rayon` appears only in `store` and `ingest`, not in `analysis`, which is the opposite of where parallel
analysis would be expected. Worth a look before the split so the dependency does not end up on the wrong
side of it.
