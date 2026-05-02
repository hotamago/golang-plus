<p align="center">
  <img src="assets/logo.png" alt="GoPlus Logo" width="200" />
</p>

# goplus

`goplus` is a surface language for Go, implemented as a Rust transpiler (`*.gp -> *.go`).

The project goal is to keep the full Go ecosystem (toolchain, runtime, packages) while improving developer ergonomics: cleaner error flow, better enum/match support, and safe compile-time metaprogramming.

## Vision

`goplus` does not replace Go.

`goplus` is a productivity layer on top of Go that produces readable, debuggable, and review-friendly generated Go code.

Long-term vision:
- Become a practical language layer for real Go teams.
- Keep generated Go transparent to avoid lock-in.
- Build a strong contributor community around compiler, diagnostics, tooling, and language design.

## Core Goals

- Practical compatibility with Go toolchain.
- Go-like syntax with low learning curve.
- Concise error handling via `!` and `?`.
- Strong enum/match support, including exhaustive checks.
- Flexible compile-time decorators, including user-defined decorators.
- Human-readable generated code, always formatted with `gofmt`.

## Current Non-Goals

- Rust-style borrow checker.
- Free-form token-level macro system.
- Custom runtime replacing Go runtime.
- Overly complex type system that hurts simplicity.

## Current Status (v1)

- Syntax: `fn`, `struct`, `enum` (simple + tagged generic), `impl`.
- Error sugar: `-> T!`, `-> !`, `expr?`.
- `match` with enum exhaustive checking.
- `@derive(String, Debug, Equal, JsonMarshal, JsonUnmarshal)` for struct/enum.
- Package compilation:
  - Standalone `.gp` files outside a Go module compile as the selected file.
  - Directories and Go module packages compile all sibling `.gp` files together.
  - Sibling `.go` files are copied into the generated package so Go and GoPlus code can link together.
- Compile-time decorators:
  - Built-in: `@log`, `@retry(times[, backoff_ms])`, `@memoize`.
  - Custom decorators (Python-like factory style: `next -> wrapped`).
- CLI:
  - `goplus check`
  - `goplus transpile`
  - `goplus build`
  - `goplus run`
  - `goplus fmt` (and `goplus fmt --check`)
  - `goplus lint`
  - `goplus navigate`

## v2 Tooling & DevEx

The compiler is now organized into scalable frontend, semantic, codegen, and
project orchestration submodules under `src/parser`, `src/sema`, `src/codegen`,
and `src/compiler`. No core module is intended to grow into a thousand-line
catch-all file again.

With the completion of v2, the ecosystem now features production-grade tooling:
- **IDE Diagnostics**: Diagnostics include severity levels (`Error`, `Warning`, `Info`), codes, precise caret spans, and hints. Available via `goplus check --diagnostic-format json`.
- **Rewriting Formatter**: `goplus fmt` deterministically reformats `.gp` sources.
- **Linter**: `goplus lint` catches stylistic issues and suspicious code without compiling to Go.
- **Source Navigation**: `goplus navigate` and generated source maps enable bidirectional lookups (`.gp` ↔ `.go`).
- **Topological Builds**: The package graph uses Kahn's algorithm to resolve dependencies, transpile in topological order, and catch cycles early.
- **Strong Decorator Validation**: Full callable signature analysis ensures custom decorators perfectly match their targets.

## Compiler Architecture

- `lexer` -> `parser` -> `semantic` -> `codegen Go` -> `gofmt`.
- Semantic layer enforces key rules: decorator contracts, `?` context, exhaustive match.
- Codegen prioritizes readability and debuggability over micro-optimizations.

## Quick Start

First, download and install `goplus` from the [Releases](https://github.com/hotamago/golang-plus/releases) page (Windows users can use the `.msi` installer). Then run:

```bash
goplus check examples/demo.gp
goplus transpile examples/demo.gp --out-dir .goplusgen
goplus run examples/demo.gp --out-dir .goplusgen
```

`goplus` runs a standalone `.gp` path outside a Go module as the selected file.
If you point it at a directory, or at a file inside a Go module package, it will compile every `*.gp` file in that package and link any sibling `*.go` files.

Mixed-source example:

```bash
goplus run examples/link-source/main.gp --out-dir .goplusgen
```

The `examples/link-source` sample now includes a real `go.mod` plus nested `pkg/...` and `internal/...` packages, so it shows both same-package linking and normal imported package boundaries.
Most of that example is written in `.gp`; it keeps only one `.go` bridge file to demonstrate GoPlus/Go interop explicitly.

## Short Example

```gp
package main

import "fmt"

@derive(String)
enum Status {
    Pending
    Running
    Done
}

@log
@retry(3, 10)
fn readName() -> string! {
    return "goplus"
}

fn main() -> ! {
    name := readName()?
    fmt.Println(name)
    fmt.Println(Status::Running)
    return
}
```

## Custom Decorators

A custom decorator is a function that takes `next` (the previous function in the decorator chain) and returns a function with the same signature.

```gp
fn trace(next: func(path string) (string, error), label: string) -> func(path string) (string, error) {
    return func(path string) (string, error) {
        fmt.Println("trace:", label)
        return next(path)
    }
}

@trace("io")
fn load(path: string) -> string! {
    return "ok"
}
```

## VSCode Extension

GoPlus provides a native-like development experience in VSCode, powered by the `goplus` CLI.

### Features
- **Diagnostics**: Real-time error reporting with severity levels, precise spans, and hints.
- **Hover Information**: Documentation and signatures for keywords, variables, parameters, and decorators.
- **Auto-completion**: Suggestions for keywords, built-in decorators, enum variants, and derive options.
- **Formatting**: Automatic code formatting using `goplus fmt`.
- **Linting**: Code quality checks using `goplus lint`.
- **Go to Definition**: Bidirectional source navigation (`.gp` ↔ `.go`) powered by source maps.
- **Snippets**: Quick templates for `fn`, `struct`, `enum`, and decorators.

### Installation

The extension requires the `goplus` CLI to be available in your system `$PATH` (or `%PATH%` on Windows).

1. **Install CLI**:
   - **Windows**: Download the `.msi` installer from the [Releases](https://github.com/hotamago/golang-plus/releases) page and run it. The installer will automatically add `goplus` to your system `$PATH`!
   - **Other OS / From Source**:
     ```bash
     cargo install --path .
     ```

2. **Install Extension**:
   - Navigate to the `editors/vscode` directory.
   - Run `npm install` and `npm run package` (requires `vsce`) to build the extension, or use the pre-built `.vsix` file in the directory.
   - Install the generated `.vsix` file in VSCode (`Extensions: Install from VSIX...` from the command palette).

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the detailed source of truth.

Short status:
- v1.x stabilization is complete for the current syntax.
- v2 tooling is **complete**. The tooling foundation is robust, featuring deterministic formatting, linting, source maps, topological build graphs, and strong decorator validation.
- v3 VSCode Extension is **complete**. Install from `editors/vscode/` or the packaged `.vsix`.

## Contributing

New contributors are very welcome.

Quick start:
- Run tests: `cargo test`
- Run example: `cargo run -- run examples/demo.gp --out-dir .goplusgen`
- Start reading core modules:
  - `src/parser/`
  - `src/sema/`
  - `src/codegen/`
  - `src/compiler/`

Areas where contributions are especially useful:
- Parser and grammar improvements.
- Additional semantic checks + better test coverage.
- Better generated Go quality in edge cases.
- More examples/benchmarks/problem-style samples.
- Better docs for language spec and migration from Go.

Pull request principles:
- Include tests for new behavior or bug fixes.
- Do not reduce readability of generated Go without a strong reason.
- Keep v1 backward compatibility whenever possible.
