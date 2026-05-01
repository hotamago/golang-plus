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
| Diagnostics codes/source excerpts | Partial | Human diagnostics and JSON output exist; many errors still need more precise spans and suggestions. |
| Parser coverage | Partial | Block comments, grouped imports, and raw Go-like statement pass-through exist; full Go-like grammar coverage is not done. |
| Semantic checks | Partial | Duplicate declarations, generated-name collisions, match arity, duplicate match arms, and some decorator checks exist. |
| Source maps | Partial | `--emit-source-map` writes metadata; real `.gp` span to generated `.go` span mappings are not complete. |
| Formatter | Partial | `fmt --check` validates source; rewriting formatter is not implemented. |
| CI example coverage | Partial | CI checks all `.gp` example sources and builds selected example packages. Full fixture/golden matrix is still planned. |

## v1.x Stabilization

Goal: make the current language reliable, predictable, well-tested, and
performance-conscious without breaking existing `.gp` syntax.

| Task | Status | Acceptance Criteria |
| --- | --- | --- |
| Keep current syntax backward-compatible | Done | Existing examples and tests keep passing. |
| CI checks every example source | Done | GitHub Actions runs `goplus check` on every `examples/**/*.gp` outside generated output. |
| CI builds representative example packages | Done | CI builds `examples/link-source` and `examples/complex-app`. |
| Add complex integration examples | Done | `examples/complex-app` covers grouped imports, block comments, derive, impl, tagged enums, match, decorators, memoize, error sugar, and raw Go-like statements. |
| Expand diagnostic precision | Partial | Add focused spans and hints for parser recovery, decorator errors, match errors, and package/module errors. |
| Complete real source map mappings | Planned | `--emit-source-map` records useful `.gp` to generated `.go` ranges for functions, declarations, match arms, and statements. |
| Add fixture/golden test matrix | Planned | Golden tests cover parser, diagnostics, generated Go, decorator chains, tagged enum edge cases, source maps, and interop. |
| Expand parser coverage | Planned | Structured parsing for assignments, `defer`, `go`, `for`, `switch`, `select`, import aliases, and common Go declarations. |
| Make generic tagged enums build-clean | Planned | Constructors and type references emit Go type arguments with `[...]`, not GoPlus `<...>` syntax. |
| Improve generated Go robustness | Planned | Build generated Go in more fixtures, catch name collisions earlier, and keep output readable. |
| Improve transpile performance | Planned | Add benchmarks and avoid regenerating unchanged packages where possible. |

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
