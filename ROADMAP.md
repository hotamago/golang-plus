# GoPlus Roadmap

This file is the source of truth for project direction. It separates completed
foundation work from unfinished v1.x and v2 work so contributors and agents can
pick tasks without guessing.

Status labels:

- **Done**: implemented, tested, and expected to remain stable.
- **Partial**: usable foundation exists, but the feature is not complete.
- **Planned**: not implemented yet.

## Current State

GoPlus is an early compiler/transpiler for `.gp -> .go`. The current codebase has
a scalable module layout and a growing compatibility suite, but v1.x is not
complete and v2 tooling is not complete.

| Area | Status | Notes |
| --- | --- | --- |
| Compiler module split | Done | Core modules live under `src/parser`, `src/sema`, `src/codegen`, and `src/compiler`. |
| CLI `check`, `transpile`, `build`, `run` | Done | Existing workflows are preserved. |
| Diagnostics codes/source excerpts | Done | Human and JSON diagnostics include stable codes, source excerpts, spans, and hints for v1.x parser, decorator, match, and package errors. |
| Parser coverage | Done | v1.x structured coverage includes grouped/aliased imports, common statements, raw Go-like declarations, and fallback pass-through. |
| Semantic checks | Done | Duplicate declarations, enum variant/name collisions, generated-name collisions, match arity/duplicates, and decorator checks are covered. |
| Source maps | Done | `--emit-source-map` writes readable JSON with useful `.gp` to generated `.go` declaration, function, match arm, and statement ranges. |
| Formatter | Partial | `fmt --check` validates source; rewriting formatter is not implemented. |
| CI example coverage | Done | CI checks all `.gp` example sources, builds selected example packages, and compiles the benchmark suite. |

## v1.x Stabilization

Goal: make the current language reliable, predictable, well-tested, and
performance-conscious without breaking existing `.gp` syntax.

| Task | Status | Acceptance Criteria |
| --- | --- | --- |
| Keep current syntax backward-compatible | Done | Existing examples and tests keep passing. |
| CI checks every example source | Done | GitHub Actions runs `goplus check` on every `examples/**/*.gp` outside generated output. |
| CI builds representative example packages | Done | CI builds `examples/link-source` and `examples/complex-app`. |
| Add complex integration examples | Done | `examples/complex-app` covers grouped imports, block comments, derive, impl, tagged enums, match, decorators, memoize, error sugar, and raw Go-like statements. |
| Expand diagnostic precision | Done | Focused spans, stable codes, and hints cover parser recovery, decorator errors, match errors, and package/module errors. |
| Complete real source map mappings | Done | `--emit-source-map` records useful `.gp` to generated `.go` ranges for functions, declarations, match arms, and statements. |
| Add fixture/golden test matrix | Done | Fixture tests cover parser, diagnostics, generated Go, tagged enum edge cases, source maps, and build interop. |
| Expand parser coverage | Done | Structured parsing covers assignments, `defer`, `go`, `for`, `switch`, `select`, import aliases, and common Go declarations. |
| Make generic tagged enums build-clean | Done | Constructors and type references emit Go type arguments with `[...]`, not GoPlus `<...>` syntax. |
| Improve generated Go robustness | Done | Generated Go is built in more fixtures, enum/generated-name collisions are caught earlier, and output stays gofmt-readable. |
| Improve transpile performance | Done | Benchmarks exist and unchanged packages are skipped using a content-hash package cache manifest. |

## v2 Tooling And Devex

Goal: build production-grade tooling on top of the stabilized compiler frontend.
Only foundation hooks exist today; v2 is not done.

| Task | Status | Acceptance Criteria |
| --- | --- | --- |
| Formatter command shape | Partial | `fmt --check` exists; rewriting mode is planned. |
| Rewriting formatter | Planned | `goplus fmt` rewrites `.gp` files deterministically and has golden tests. |
| Linter | Planned | `goplus lint` reports style and suspicious-code diagnostics without generating Go. |
| IDE diagnostics | Partial | JSON diagnostics exist; editor-ready ranges and severity levels are planned. |
| Source navigation | Planned | Source maps support jump between `.gp` and generated `.go`. |
| Richer derives | Planned | Candidate derives: `Debug`, `Equal`, JSON helpers. |
| Strong decorator signature validation | Partial | Known local decorators get basic arity/function-type checks; full callable shape validation is planned. |
| Package graph improvements | Planned | Better local package discovery, caching, and large-project behavior. |

## Contribution Priorities

Recommended order:

1. Finish v1.x diagnostics and source map mappings.
2. Add fixture/golden tests before broadening syntax.
3. Expand structured parser coverage while preserving raw Go pass-through.
4. Build formatter/linter on the stabilized AST/span model.
5. Add richer derives and stronger decorator type validation.

For implementation work, start with:

- Parser/frontend: `src/parser/`, `src/lexer.rs`, `src/ast.rs`
- Semantics: `src/sema/`
- Generated Go: `src/codegen/`
- Project loading, CI-facing behavior, and Go toolchain integration: `src/compiler/`
