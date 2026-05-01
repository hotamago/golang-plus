# GoPlus Language Guide

This guide documents the current GoPlus language implemented in this repository.
GoPlus is a surface language for Go: source files use the `.gp` extension and the
compiler transpiles them to regular, formatted Go code.

The goal is not to replace Go. GoPlus keeps the Go toolchain, runtime, package
system, and ecosystem, while adding a small set of ergonomic features:

- `fn` syntax for functions and methods.
- Concise error returns with `!`.
- Concise error propagation with `?`.
- `struct`, `enum`, `impl`, and `match`.
- Exhaustive enum matching.
- Built-in and custom decorators.
- Package-level compilation that can link `.gp` and `.go` files.

## Contents

- [Project Status](#project-status)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Command Line Interface](#command-line-interface)
- [Source Files and Packages](#source-files-and-packages)
- [Lexical Rules](#lexical-rules)
- [Program Structure](#program-structure)
- [Imports](#imports)
- [Types](#types)
- [Functions](#functions)
- [Error Handling](#error-handling)
- [Statements](#statements)
- [Structs](#structs)
- [Enums](#enums)
- [Match](#match)
- [Impl Blocks and Methods](#impl-blocks-and-methods)
- [Decorators](#decorators)
- [Go Interoperability](#go-interoperability)
- [Generated Go](#generated-go)
- [Diagnostics](#diagnostics)
- [Current Limitations](#current-limitations)
- [Examples](#examples)
- [Contributor Notes](#contributor-notes)

## Project Status

GoPlus is currently an early v1 implementation. It is useful for experimenting
with Go-like syntax, error sugar, enum/match support, decorators, and mixed
GoPlus/Go packages.

Implemented today:

- Top-level declarations: `fn`, `struct`, `enum`, `impl`.
- Imports using Go import paths.
- Return types: no return, `T`, `!`, and `T!`.
- `?` on expressions that return an error.
- `match` over simple and tagged enums.
- Exhaustiveness checks for enum matches.
- `@derive(String)` on structs and enums.
- Built-in decorators: `@log`, `@retry(times[, backoff_ms])`, `@memoize`.
- Custom decorators using decorator factory functions.
- Package compilation:
  - A `.gp` file compiles all sibling `.gp` files in the same directory.
  - Sibling `.go` files can be copied into the generated package.
  - Go modules are mirrored into the output directory when a `go.mod` is found.

Non-goals for the current implementation:

- A custom runtime.
- A borrow checker.
- A free-form token macro system.
- A complex type system that diverges heavily from Go.

## Requirements

You need:

- Rust and Cargo, because the compiler is implemented in Rust.
- Go, because GoPlus emits Go and uses `go`, `gofmt`, `go build`, and `go run`.

From the repository root, the compiler can be run with Cargo:

```bash
cargo run -- check examples/demo.gp
```

The package name in `Cargo.toml` is `goplus`, and the CLI command name is also
`goplus`.

## Quick Start

Create a file named `main.gp`:

```gp
package main

import "fmt"

fn main() -> ! {
    fmt.Println("hello from goplus")
    return
}
```

Run it through the compiler:

```bash
cargo run -- run main.gp --out-dir .goplusgen
```

GoPlus generates Go into `.goplusgen`, formats it with `gofmt`, then delegates
execution to the Go toolchain.

## Command Line Interface

The CLI has four subcommands.

### `check`

Parses and semantically checks a GoPlus source file or source directory.

```bash
cargo run -- check examples/demo.gp
```

`check` does not write generated Go. It verifies syntax, semantic rules, package
consistency, decorator constraints, valid `?` usage, and enum match
exhaustiveness.

Diagnostics can also be emitted as JSON:

```bash
cargo run -- check examples/demo.gp --diagnostic-format json
```

### `transpile`

Transpiles GoPlus to Go and writes generated files to an output directory.

```bash
cargo run -- transpile examples/demo.gp --out-dir .goplusgen
```

The default output directory is `.goplusgen`.

To write source map JSON beside the generated Go file:

```bash
cargo run -- transpile examples/demo.gp --out-dir .goplusgen --emit-source-map
```

### `build`

Transpiles the package and then runs `go build`.

```bash
cargo run -- build examples/demo.gp --out-dir .goplusgen
```

You can choose the output binary path:

```bash
cargo run -- build examples/demo.gp --out-dir .goplusgen --out ./demo
```

On Windows, the default binary name uses `.exe`.

### `run`

Transpiles the package and then runs it with `go run`.

```bash
cargo run -- run examples/demo.gp --out-dir .goplusgen
```

### `fmt`

The formatter command is currently a validation hook. It parses and checks
GoPlus source without rewriting files:

```bash
cargo run -- fmt examples/demo.gp --check
```

## Source Files and Packages

GoPlus source files use the `.gp` extension.

When you pass a `.gp` file to the compiler, GoPlus treats that file as an entry
point into its containing package directory. It compiles every sibling `.gp`
file in that same directory as one Go package.

For example:

```text
app/
  main.gp
  messages.gp
  bridge.go
```

Running:

```bash
cargo run -- run app/main.gp --out-dir .goplusgen
```

compiles both `main.gp` and `messages.gp`. If `bridge.go` exists, it is copied
into the generated package so generated Go and handwritten Go can link together.

All `.gp` files in one package directory must declare the same package name.

You may also pass a directory instead of a single `.gp` file:

```bash
cargo run -- check app
```

The directory must contain at least one `.gp` file.

## Go Module Behavior

If the compiler finds a `go.mod` in the source directory or one of its parent
directories, it treats that directory as the module root.

In module mode, GoPlus:

- Copies `go.mod` and `go.sum` into the output module root when present.
- Copies native `.go` files recursively into the output module tree.
- Compiles the entry package into the matching relative package path.
- Follows local imports inside the module when imported packages contain `.gp`
  files.

Example module layout:

```text
examples/link-source/
  go.mod
  main.gp
  messages.gp
  bridge.go
  pkg/support/support.gp
  internal/label/label.gp
```

Run it from the repository root:

```bash
cargo run -- run examples/link-source/main.gp --out-dir .goplusgen
```

The compiler mirrors the module layout under `.goplusgen` and lets `go run`
execute the generated module package.

## Lexical Rules

GoPlus source uses a compact Go-like lexical model.

Whitespace:

- Spaces, tabs, carriage returns, and form feeds are skipped.
- Newlines are tokens and can separate declarations and statements.
- Semicolons can also act as separators.

Comments:

- Line comments starting with `//` are supported.
- Block comments are not part of the current lexer.

Literals:

- String literals use double quotes, for example `"hello"`.
- Integer literals can contain underscores, for example `1_000_000`.

Identifiers:

- Identifiers follow the pattern `[A-Za-z_][A-Za-z0-9_]*`.

Reserved keywords:

```text
package import fn struct enum impl match return if else for mut self type
```

Operators and punctuation recognized by the parser include:

```text
-> => :: := @ ! ? : ; , . = ( ) { } [ ] < > + - * / % & |
```

Most expression text is preserved and emitted as Go after a few GoPlus-specific
rewrites, such as enum constructors and `fn(...) -> ...` function types.

## Program Structure

Every GoPlus file starts with a package declaration:

```gp
package main
```

Imports come after the package declaration:

```gp
import "fmt"
import "example.com/project/pkg/support"
```

Top-level declarations come after imports:

```gp
struct User {
    Name: string
}

enum Status {
    Pending
    Running
    Done
}

fn main() -> ! {
    return
}
```

Supported top-level declarations:

- `struct`
- `enum`
- `fn`
- `impl`

Decorators can appear before `fn`, methods inside `impl`, `struct`, and `enum`,
subject to the decorator rules described later.

## Imports

Imports use Go import path strings:

```gp
import "fmt"
import "strconv"
import "example.com/goplus-link-source/pkg/support"
```

GoPlus emits these imports into the generated Go file. The generator may add
extra imports when a language feature needs them:

- `fmt` for `@log`, `@derive(String)`, try helper errors, or the generated
  `main` wrapper.
- `os` for the generated `main` wrapper.
- `time` for `@retry`.
- `sync` for `@memoize`.
- `errors` for `error("message")` return sugar.

Imports are emitted in sorted order by the generated Go code.

## Types

GoPlus intentionally reuses Go's type model where possible. Many type
references are passed through directly to generated Go:

```gp
fn count(items: []string) -> int {
    return len(items)
}

fn lookup(values: map[string]int, key: string) -> int {
    return values[key]
}
```

### Function Types

GoPlus accepts Go function types:

```gp
fn trace(next: func(path string) (string, error)) -> func(path string) (string, error) {
    return next
}
```

It also accepts GoPlus-style function types:

```gp
fn trace(next: (fn(path: string) -> string!)) -> (fn(path: string) -> string!) {
    return next
}
```

The compiler rewrites GoPlus-style function types to Go function types:

```go
func(path string) (string, error)
```

Return mappings for function types:

- `fn() -> !` becomes `func() error`.
- `fn() -> T!` becomes `func() (T, error)`.
- `fn() -> T` becomes `func() T`.

### Generics

Functions and enums can declare type parameters using angle brackets:

```gp
fn identity<T>(value: T) -> T {
    return value
}

enum Result<T, E> {
    Ok(T)
    Err(E)
}
```

Generated Go uses type parameters constrained with `any`.

## Functions

Functions use `fn`:

```gp
fn add(a: int, b: int) -> int {
    return a + b
}
```

Parameters use `name: Type` syntax. Return types are introduced by `->`.

Return forms:

```gp
fn logOnly(message: string) {
    fmt.Println(message)
}

fn name() -> string {
    return "goplus"
}

fn save() -> ! {
    return
}

fn load() -> string! {
    return "ok"
}
```

Generated Go return types:

| GoPlus | Generated Go |
| --- | --- |
| no `->` | no return values |
| `-> T` | `T` |
| `-> !` | `error` |
| `-> T!` | `(T, error)` |

### `main` Returning `!`

If `main` returns `!`, the generator emits the real logic as `mainWarp` and
creates a Go `main` wrapper that prints the error to stderr and exits with code
1.

GoPlus:

```gp
fn main() -> ! {
    return
}
```

Generated shape:

```go
func mainWarp() error {
    return nil
}

func main() {
    if err := mainWarp(); err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }
}
```

## Error Handling

GoPlus adds two pieces of error sugar: `!` in return types and `?` on
expressions.

### Error-Capable Functions

A function is error-capable if it returns:

- `!`
- `T!`

Only error-capable functions may use `?`.

```gp
fn readName() -> string! {
    return "goplus"
}

fn main() -> ! {
    name := readName()?
    fmt.Println(name)
    return
}
```

For a variable declaration with `?`, the generated Go checks the returned error:

```gp
name := readName()?
```

becomes roughly:

```go
name, __gp_err0 := readName()
if __gp_err0 != nil {
    return __gp_err0
}
```

Inside a `T!` function, errors return the zero value for `T` plus the error.

### Returning Values from `T!`

In a `T!` function, returning a plain `T` value automatically appends `nil`:

```gp
fn readName() -> string! {
    return "goplus"
}
```

Generated shape:

```go
func readName() (string, error) {
    return "goplus", nil
}
```

### Returning Errors

GoPlus recognizes `error("message")` in return position and maps it to
`errors.New("message")`.

```gp
fn fail() -> string! {
    return error("not found")
}
```

Generated shape:

```go
func fail() (string, error) {
    return "", errors.New("not found")
}
```

For `-> !`, a bare `return` becomes `return nil`.

### `?` in Expression Statements and Returns

For expression statements and `return expr?`, GoPlus uses a helper when it only
needs to inspect the final error return value.

```gp
fn run() -> ! {
    doSomething()?
    return
}
```

The generated helper checks that the last returned value is an `error`.

## Statements

GoPlus parses a small set of statements directly and lets many Go statements
pass through as raw Go.

Directly supported statements:

- Variable declaration with `:=`.
- `return`.
- `if` / `else if` / `else`.
- `match`.
- Expression statements.
- Raw `for` statements.

### Variable Declarations

```gp
name := readName()?
count := len(name)
```

Variable declarations use `:=`, matching Go.

### Return

```gp
return
return value
return value, err
```

In `T!` functions, `return value` becomes `return value, nil`.

### If and Else

```gp
if value > 10 {
    fmt.Println("large")
} else if value > 0 {
    fmt.Println("positive")
} else {
    fmt.Println("zero or negative")
}
```

Conditions are emitted as Go expressions.

### For

`for` statements are treated as raw Go syntax and emitted through the generator.

```gp
i := 0
for i < 10 {
    fmt.Println(i)
    i += 1
}
```

### Raw Go Statements

Many Go statements and expressions can be written directly in `.gp` files when
they do not conflict with GoPlus parsing.

For example:

```gp
defer out.Flush()
continue
panic("unexpected EOF")
```

GoPlus is intentionally permissive here because it is a surface language over
Go, but not every Go syntax form has dedicated parser support yet.

## Structs

Struct declarations use `struct`:

```gp
struct User {
    Name: string
    Age: int
}
```

Generated Go:

```go
type User struct {
    Name string
    Age  int
}
```

Fields use `name: Type` syntax. Fields may be separated by newlines or commas.
Use exported Go field names, such as `Name`, when other packages need access.

### Deriving String for Structs

`@derive(String)` generates a `String() string` method:

```gp
@derive(String)
struct User {
    Name: string
    Age: int
}
```

The generated implementation uses `fmt.Sprintf`.

Only `String` is supported as a derive target in v1.

## Enums

GoPlus supports simple enums and tagged enums.

### Simple Enums

A simple enum has variants without payloads:

```gp
@derive(String)
enum Status {
    Pending
    Running
    Done
}
```

Generated Go shape:

```go
type Status int

const (
    StatusPending Status = iota
    StatusRunning
    StatusDone
)
```

Use variants with `EnumName::Variant` in GoPlus:

```gp
fmt.Println(Status::Running)
```

The generator rewrites this to the generated Go constant name:

```go
fmt.Println(StatusRunning)
```

With `@derive(String)`, simple enums get a `String() string` method.

### Tagged Enums

A tagged enum has one or more variants with payloads:

```gp
enum Result<T, E> {
    Ok(T)
    Err(E)
}
```

Generated Go uses:

- An internal tag enum.
- A struct containing the tag and payload fields.
- Constructor functions for each variant.

Create tagged enum values using `EnumName::Variant(...)`:

```gp
value := Result<string, error>::Ok("done")
```

The generator rewrites this to a constructor call:

```go
ResultOk[string, error]("done")
```

Payload fields are stored internally and are intended to be accessed through
`match`, not directly.

With `@derive(String)`, tagged enums get a `String() string` method that prints
the variant and payload values.

## Match

`match` compiles to a Go `switch`.

Simple enum example:

```gp
fn statusText(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
        Status::Done => "done",
    }
}
```

For non-void functions, expression arms return their expression automatically.
The example above becomes a switch with `return` in each case.

### Patterns

Supported patterns:

```gp
Status::Pending
Pending
Result::Ok(value)
Ok(value)
_
```

Typed patterns name the enum explicitly:

```gp
Status::Pending
```

Untyped patterns are allowed when the compiler can resolve the enum type from
the match expression or from the set of variants:

```gp
match s {
    Pending => "pending",
    Running => "running",
    Done => "done",
}
```

Wildcard `_` acts as the default case.

### Tagged Enum Bindings

Tagged enum payloads can be bound in patterns:

```gp
fn describe(r: Result<string, error>) -> string {
    match r {
        Result::Ok(value) => value,
        Result::Err(err) => err.Error(),
    }
}
```

Use `_` to ignore a payload value:

```gp
Result::Ok(_) => "ok"
```

### Exhaustiveness

If a `match` resolves to an enum and has no wildcard arm, all variants must be
covered.

This is invalid:

```gp
fn statusText(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
    }
}
```

The semantic analyzer reports a non-exhaustive match and lists missing variants.

Add the missing variants or a wildcard:

```gp
fn statusText(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
        _ => "other",
    }
}
```

The generated Go still includes an `unreachable` default for exhaustive enum
matches without a wildcard.

## Impl Blocks and Methods

Methods are declared inside `impl` blocks:

```gp
struct User {
    Name: string
}

impl User {
    fn greet(self) -> string {
        return "hello " + self.Name
    }
}
```

The first method parameter must be one of:

- `self` for a value receiver.
- `mut self` for a pointer receiver.

Pointer receiver example:

```gp
impl Counter {
    fn inc(mut self) {
        self.Value += 1
    }
}
```

Generated Go shape:

```go
func (self *Counter) inc() {
    self.Value += 1
}
```

Additional method parameters use normal `name: Type` syntax:

```gp
impl User {
    fn rename(mut self, name: string) {
        self.Name = name
    }
}
```

Decorators may be attached to methods, except for decorators whose rules reject
methods, such as `@memoize`.

Decorators are not supported directly on `impl` blocks.

## Decorators

Decorators are annotations that wrap functions or methods at compile time.

```gp
@log
@retry(3, 100)
fn load(path: string) -> string! {
    return readFile(path)?
}
```

Decorators are applied as a chain. The generator emits:

- An inner function containing the original body.
- One wrapper per decorator.
- A final forwarding function with the original public name.

### `@log`

`@log` logs function entry and exit using `fmt.Printf`.

```gp
@log
fn greet(name: string) -> string {
    return "hello " + name
}
```

For error-capable functions, it logs errors before returning them.

### `@retry`

`@retry` retries error-capable functions.

```gp
@retry(3)
fn load() -> string! {
    return readName()?
}
```

With backoff:

```gp
@retry(3, 100)
fn load() -> string! {
    return readName()?
}
```

Rules:

- The decorated function must return `!` or `T!`.
- The first argument is the number of attempts.
- The optional second argument is backoff in milliseconds.
- Arguments must be positive integers.

Generated Go uses `time.Sleep` when backoff is greater than zero.

### `@memoize`

`@memoize` caches results for top-level functions with a non-error return type.

```gp
@memoize
fn add(a: int, b: int) -> int {
    return a + b
}
```

Rules:

- Only top-level functions are supported.
- Methods cannot use `@memoize`.
- The function must return `T`, not `!` or `T!`.
- Parameters must be comparable, because they are used as map keys.
- Slices, maps, and function types are rejected as parameters.

Generated Go uses a key struct, a map, and a `sync.Mutex`.

### Custom Decorators

A custom decorator is a function that receives the next function in the chain
and returns a function with the same callable shape.

Go-style function type example:

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

GoPlus-style function type example:

```gp
fn trace(next: (fn(name: string) -> string!), label: string) -> (fn(name: string) -> string!) {
    return fn(name: string) -> string! {
        fmt.Println("trace:", label)
        return next(name)
    }
}

@trace("custom")
fn greet(name: string) -> string! {
    return "hello " + name
}
```

The compiler rewrites `fn(...) -> ...` function literals and function types into
Go `func(...) ...` syntax in generated output.

Custom decorator names may include dot-qualified names:

```gp
@decorators.trace("svc")
fn run() {
    return
}
```

The semantic analyzer allows unknown decorator names as custom decorators.

## Go Interoperability

GoPlus is designed to interoperate with Go.

### Calling Go from GoPlus

Import Go packages normally:

```gp
package main

import "fmt"
import "strconv"

fn atoiStrict(s: string) -> int! {
    return strconv.Atoi(s)?
}

fn main() -> ! {
    n := atoiStrict("42")?
    fmt.Println(n)
    return
}
```

### Linking Same-Package `.go` Files

If a `.go` file sits next to `.gp` files in the same package directory, GoPlus
can copy it into the generated package.

Example:

```text
app/
  main.gp
  bridge.go
```

`main.gp`:

```gp
package main

import "fmt"

fn main() -> ! {
    fmt.Println(messageFromGo())
    return
}
```

`bridge.go`:

```go
package main

func messageFromGo() string {
    return "hello from Go"
}
```

Because both files are in `package main`, they share one package namespace.

### Calling GoPlus from Go

Generated Go functions are normal Go symbols. Handwritten Go in the same package
can call functions generated from `.gp` files.

Example from the repository:

```gp
package main

fn fromGpFile() -> string {
    return "same-folder .gp files share one package namespace"
}
```

A sibling Go file can call `fromGpFile()` after transpilation.

### Imported GoPlus Packages

Inside a Go module, imported packages that contain `.gp` files are compiled into
matching generated package directories.

Example:

```gp
import "example.com/goplus-link-source/pkg/support"
```

If `pkg/support` contains `.gp` files, the compiler emits generated Go for that
package under the output module root.

## Generated Go

Generated Go is intended to be readable and debuggable.

The compiler pipeline is:

```text
lexer -> parser -> semantic analysis -> Go codegen -> gofmt
```

Generated package file name:

```text
zz_goplus_gen.go
```

The generator always runs `gofmt` on generated Go files.

For non-module packages, sibling `.go` files are copied into the output package
directory. For module packages, `.go` files are copied recursively while
skipping generated/cache directories.

The compiler uses package-local Go cache directories during `build` and `run`:

- `.gocache`
- `.gotmp`

These live inside the generated package directory.

## Diagnostics

GoPlus reports diagnostics for syntax and semantic errors with diagnostic codes,
file context, source excerpts, caret spans, and hints when available. Use
`--diagnostic-format json` on `check` when editor or tooling integrations need a
machine-readable result.

Examples of checked errors:

- Invalid tokens.
- Missing package declarations.
- Invalid imports.
- Mismatched package names across sibling `.gp` files.
- Unsupported annotations on type declarations.
- Unsupported derive targets.
- Decorators placed on unsupported declarations.
- `?` used outside functions returning `!` or `T!`.
- `@retry` on non-error-returning functions.
- Invalid `@retry` arguments.
- `@memoize` on methods.
- `@memoize` on functions with non-comparable parameters.
- Unknown enum variants in `match`.
- Match arms using the wrong enum.
- Non-exhaustive enum matches.
- Match expressions whose enum type cannot be resolved.

When a match expression cannot be resolved, use explicit typed patterns:

```gp
match s {
    Status::Pending => "pending",
    Status::Running => "running",
    Status::Done => "done",
}
```

## Current Limitations

The current implementation is intentionally small. Important limitations:

- Parser coverage is stabilized for v1.x syntax, with raw passthrough for
  unsupported Go-like forms.
- `for`, `switch`, and `select` statements are recognized and emitted as raw Go.
- `goplus fmt --check` exists, but there is no rewriting GoPlus formatter yet.
- `--emit-source-map` writes readable JSON mappings for declarations,
  functions, match arms, and statements.
- `@derive` supports only `String`.
- `@memoize` supports only top-level functions returning `T`.
- Decorator signature compatibility is mostly enforced by generated Go and the
  Go compiler, not fully by GoPlus semantic analysis.
- Method type parameters are parsed, but generated Go method signatures do not
  emit method type parameters.
- Exhaustive `match` applies when the semantic analyzer can resolve the enum.
- Tagged enum payload fields are implementation details of generated Go.
- The generated wrapper for `main -> !` currently uses the internal name
  `mainWarp`.

When writing GoPlus today, prefer straightforward Go-like code and use generated
Go as the final compatibility check.

## Examples

### Basic Program

```gp
package main

import "fmt"

fn main() -> ! {
    fmt.Println("hello")
    return
}
```

Run:

```bash
cargo run -- run examples/demo.gp --out-dir .goplusgen
```

### Enum and Match

```gp
package main

@derive(String)
enum Status {
    Pending
    Running
    Done
}

fn statusText(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
        Status::Done => "done",
    }
}
```

### Error Propagation

```gp
package main

import "strconv"

fn atoiStrict(s: string) -> int! {
    return strconv.Atoi(s)?
}

fn parseAnswer() -> int! {
    value := atoiStrict("42")?
    return value
}
```

### Struct, Impl, and Method

```gp
package main

struct Counter {
    Value: int
}

impl Counter {
    fn inc(mut self) {
        self.Value += 1
    }

    fn label(self) -> string {
        return "counter"
    }
}
```

### Decorator Chain

```gp
package main

@log
@retry(3, 10)
fn readName() -> string! {
    return "goplus"
}
```

### Mixed GoPlus and Go Package

GoPlus file:

```gp
package main

import "fmt"

fn main() -> ! {
    fmt.Println(callFromGoFile())
    return
}
```

Go file:

```go
package main

func callFromGoFile() string {
    return "hello from handwritten Go"
}
```

Build:

```bash
cargo run -- build ./path/to/main.gp --out-dir .goplusgen
```

## Contributor Notes

Start with these modules:

- `src/lexer.rs` defines tokens and lexing.
- `src/parser.rs` builds the AST from `.gp` source.
- `src/ast.rs` defines the AST.
- `src/sema.rs` validates semantic rules and builds the semantic model.
- `src/codegen.rs` emits Go.
- `src/compiler.rs` handles package loading, module output, `gofmt`, and Go
  command execution.
- `src/main.rs` defines the CLI.

Useful commands:

```bash
cargo test
cargo run -- check examples/demo.gp
cargo run -- transpile examples/demo.gp --out-dir .goplusgen
cargo run -- run examples/link-source/main.gp --out-dir .goplusgen
```

When changing the language, update this guide together with parser, semantic,
codegen, and example changes.
