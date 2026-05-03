# Changelog

All notable changes to the GoPlus Language Support extension will be documented in this file.

## [0.2.0] — 2026-05-03

### Added
- **Variable Type Inlay Hints**: Implemented inline variable type hints for `let` and `:=` declarations, as well as `for ... range` loops, giving you a Rust-like experience of seeing inferred types directly in your editor.
- **Strict Type Validation**: Upgraded the `goplus check` background process to run Go's strict type checker (`go build`) on the transpiled output, mapping accurate type errors back to your `.gp` source files using source maps.

### Improved
- **Syntax Highlighting**: Added colorization for `map` types and `case`, `default`, `range` control flow keywords.
- **Hover Hints**: Added comprehensive markdown hover documentation for standard Go/GoPlus syntax (`for`, `switch`, `case`, `default`, `range`, `if`, `else`, `defer`, `go`, `select`, `int`, `string`, `map`, `bool`, `error`).


## [0.1.0] — 2026-05-02

### Added

- **Syntax highlighting**: Full TextMate grammar for `.gp` files covering keywords, types, strings, comments, decorators, enum variants, operators, and numbers.
- **Inline diagnostics**: Errors and warnings from `goplus check` and `goplus lint` display as squiggly underlines with severity, code, and hint.
- **Lint on save**: `goplus lint` runs automatically on save and populates the Problems panel.
- **Check on save**: `goplus check` runs automatically on save for parse and semantic errors.
- **Go source linking**: Click-to-navigate from `.gp` source to the corresponding generated `.go` location using source maps.
- **Reverse navigation**: Click-to-navigate from generated `.go` back to the original `.gp` source.
- **Format on save**: `goplus fmt` integrates as a VSCode formatting provider with undo/redo support.
- **Snippet support**: Built-in snippets for `fn`, `fne`, `main`, `struct`, `enum`, `match`, `impl`, `@derive`, `@log`, `@retry`, `@memoize`, `iferr`, `import`, `decfn`, and more.
- **Hover information**: Contextual documentation for decorators, derive kinds, `?`/`!` operators, `::` variant access, and GoPlus keywords.
- **Commands**: Manual `GoPlus: Check`, `Lint`, `Format`, and `Navigate` commands via the Command Palette.
- **Configuration**: Settings for binary path, output directory, and toggle switches for lint/check/format on save.
