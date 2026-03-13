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
- `@derive(String)` for struct/enum.
- Compile-time decorators:
  - Built-in: `@log`, `@retry(times[, backoff_ms])`, `@memoize`.
  - Custom decorators (Python-like factory style: `next -> wrapped`).
- CLI:
  - `goplus check`
  - `goplus transpile`
  - `goplus build`
  - `goplus run`

## Compiler Architecture

- `lexer` -> `parser` -> `semantic` -> `codegen Go` -> `gofmt`.
- Semantic layer enforces key rules: decorator contracts, `?` context, exhaustive match.
- Codegen prioritizes readability and debuggability over micro-optimizations.

## Quick Start

```bash
cargo run -- check examples/demo.gp
cargo run -- transpile examples/demo.gp --out-dir .goplusgen
cargo run -- run examples/demo.gp --out-dir .goplusgen
```

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

## Roadmap

### Near-term (v1.x)

- Improve diagnostics quality (error spans, better suggestions).
- Expand parser coverage for practical Go-like syntax.
- Increase test matrix for decorator chains, tagged enum edge cases, and interop.
- Improve transpile performance for larger projects.

### Mid-term (v2)

- Multi-file module-level compilation for `*.gp` projects.
- Tooling support: formatter/lint for goplus source.
- Better IDE/devex (source mapping and navigation between `.gp` and generated `.go`).
- Richer derive set and stronger decorator signature validation.

## Contributing

New contributors are very welcome.

Quick start:
- Run tests: `cargo test`
- Run example: `cargo run -- run examples/demo.gp --out-dir .goplusgen`
- Start reading core modules:
  - `src/parser.rs`
  - `src/sema.rs`
  - `src/codegen.rs`
  - `src/compiler.rs`

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
