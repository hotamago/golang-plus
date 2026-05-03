# GoPlus Guide for Coding Agents

**GoPlus** is a new surface language for Go. Assume your base model was not
trained on it. Treat `.gp` files as source files that compile to normal Go.

Use this file as a standalone working guide for reading, writing, checking, and
running GoPlus code in any project that contains `.gp` files. When in doubt,
transpile the current source and inspect the generated Go in the chosen output
directory, usually `.goplusgen`.

## Mental Model

- GoPlus keeps the Go runtime, Go package system, Go modules, and Go ecosystem.
- The compiler is a CLI named `goplus`.
- Source files are `*.gp`; generated Go is usually written to `.goplusgen`.
- `goplus` checks `.gp` files, transpiles them to formatted Go, then can
  delegate to `go build` or `go run`.
- Generated Go should be readable. When unsure, run `goplus transpile` and
  inspect `.goplusgen/**/zz_goplus_gen.go`.
- Prefer ordinary Go-like code. Use GoPlus features only when they are described
  in this guide or already used by nearby `.gp` files in the target project.

## CLI

From an installed CLI:

```bash
goplus check path/to/file-or-dir
goplus check path/to/file-or-dir --diagnostic-format json
goplus transpile path/to/file-or-dir --out-dir .goplusgen
goplus transpile path/to/file-or-dir --out-dir .goplusgen --emit-source-map
goplus build path/to/file-or-dir --out-dir .goplusgen
goplus build path/to/file-or-dir --out-dir .goplusgen --out ./app
goplus run path/to/file-or-dir --out-dir .goplusgen
goplus fmt path/to/file.gp
goplus fmt path/to/file.gp --check
goplus fmt path/to/file.gp --stdout
goplus lint path/to/file-or-dir
goplus lint path/to/file-or-dir --diagnostic-format json
```

Source-map navigation:

```bash
goplus navigate --source-map .goplusgen/zz_goplus_gen.go.map --file path/file.gp --line 10 --column 1
goplus navigate --source-map .goplusgen/zz_goplus_gen.go.map --file zz_goplus_gen.go --line 20 --column 1 --reverse
```

Always run at least `goplus check` after editing `.gp` files. For runnable
programs, also run `goplus run` or `goplus build`.

## Packages and Go Interop

Every file starts with a Go package declaration:

```gp
package main
```

Imports use normal Go import path strings:

```gp
import "fmt"
import "strconv"
import "example.com/project/pkg/support"
```

Go compatibility:

- GoPlus can use Go standard library packages and third-party Go modules.
- Manage dependencies with normal Go tooling, such as `go.mod`, `go get`, and
  `go mod tidy`.
- For third-party packages, add them the normal Go way, then import them from
  `.gp` files:

```bash
go get github.com/example/lib
go mod tidy
```

- Use Go package names, exported symbols, methods, interfaces, structs, maps,
  slices, channels, goroutines, and ordinary Go expressions from `.gp` code.
- GoPlus emits Go and then uses the Go toolchain, so final compatibility is
  checked by generated Go plus `go build` or `go run`.
- If a Go API returns `(T, error)` or `error`, call it with `?` from a GoPlus
  function returning `T!` or `!`.

Example using Go libraries:

```gp
package main

import "fmt"
import "net/http"
import "strconv"

fn fetchStatus(url: string) -> int! {
    resp := http.Get(url)?
    defer resp.Body.Close()
    return resp.StatusCode
}

fn parseCount(raw: string) -> int! {
    return strconv.Atoi(raw)?
}

fn main() -> ! {
    fmt.Println(parseCount("42")?)
    fmt.Println(fetchStatus("https://example.com")?)
    return
}
```

Package behavior:

- A standalone `.gp` file outside a Go module compiles as that selected file.
- A directory, or a file inside a Go module package, compiles sibling `.gp`
  files in the same package together.
- Sibling `.go` files are copied into the generated package, so `.gp` and `.go`
  code can call each other when they share the same `package`.
- In module mode, local imported packages that contain `.gp` files are compiled
  into matching generated package paths.

## Core Syntax

Functions use `fn` (or standard Go `func`); parameters are `name: Type` (or `name Type`); return types use `->` (or standard Go returns).

```gp
fn add(a: int, b: int) -> int {
    return a + b
}

fn logOnly(message: string) {
    fmt.Println(message)
}
```

Error-capable returns:

```gp
fn save() -> ! {
    return
}

fn loadName() -> string! {
    return "goplus"
}
```

Return mapping:

| GoPlus | Generated Go |
| --- | --- |
| no `->` | no return values |
| `-> T` | `T` |
| `-> !` | `error` |
| `-> T!` | `(T, error)` |

`main -> !` is allowed; generated Go wraps it and exits non-zero on error.

## Error Handling

Use `?` only inside functions returning `!` or `T!`.

```gp
fn atoiStrict(s: string) -> int! {
    return strconv.Atoi(s)?
}

fn main() -> ! {
    n := atoiStrict("42")?
    fmt.Println(n)
    return
}
```

Rules:

- `expr?` expands to Go error checking.
- In `T!`, `return value` becomes `return value, nil`.
- In `-> !`, bare `return` becomes `return nil`.
- `return error("message")` becomes an `errors.New(...)` style return.

## Structs and Methods

Struct fields use `Name: Type`.

```gp
struct User {
    Name: string
    Email: string
}
```

Methods live in `impl Type` blocks.

```gp
impl User {
    fn Display(self) -> string {
        return self.Name + " <" + self.Email + ">"
    }

    fn Rename(mut self, name: string) {
        self.Name = name
    }
}
```

- `self` generates a value receiver.
- `mut self` generates a pointer receiver.
- Additional parameters use normal `name: Type` syntax.

## Enums and Match

Simple enum:

```gp
@derive(String)
enum Status {
    Pending
    Running
    Done
}
```

Use variants as `Enum::Variant`:

```gp
fmt.Println(Status::Running)
```

Tagged generic enum:

```gp
enum Result<T> {
    Ok(T)
    Err(string)
}

fn load(ok: bool) -> Result<string> {
    if ok {
        return Result<string>::Ok("done")
    }
    return Result<string>::Err("missing")
}
```

`match` compiles to Go `switch` and can be exhaustive over enums:

```gp
fn render(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
        Status::Done => "done",
    }
}

fn describe(r: Result<string>) -> string {
    match r {
        Ok(value) => value,
        Err(reason) => "error: " + reason,
    }
}
```

Patterns may be typed (`Status::Done`), untyped (`Done`), tagged
(`Ok(value)`), or wildcard (`_`). Prefer typed patterns if diagnostics cannot
resolve the enum.

## Decorators and Derives

Decorators are compile-time wrappers placed above functions or methods.

Built-ins:

- `@log`: logs entry/exit and errors.
- `@retry(times)` or `@retry(times, backoff_ms)`: retries functions returning
  `!` or `T!`.
- `@memoize`: caches top-level functions returning non-error `T`; parameters
  must be comparable.

Example:

```gp
@log
@retry(3, 10)
fn readName() -> string! {
    return "goplus"
}
```

Custom decorators are functions that take `next` and return a function with the
same callable shape:

```gp
fn trace(next: func(name string) (string, error), label: string) -> func(name string) (string, error) {
    return func(name string) (string, error) {
        fmt.Println("trace:", label)
        return next(name)
    }
}

@trace("custom")
fn greet(name: string) -> string! {
    return "hello " + name
}
```

Supported derives include:

```gp
@derive(String, Debug, Equal, JsonMarshal, JsonUnmarshal)
```

`@derive(JSON)` is accepted as shorthand for JSON marshal and unmarshal.

## Raw Go Passthrough

GoPlus intentionally allows many Go-like statements as raw text:

```gp
go worker()

select {
case <-time.After(10 * time.Millisecond):
    fmt.Println("timer fired")
}

switch len("go") {
case 2:
    fmt.Println("two letters")
default:
    fmt.Println("other")
}
```

If a construct is not explicitly documented, try the normal Go spelling, then
validate with `goplus check` and generated Go.

## Agent Workflow

When editing GoPlus:

1. Search the target project first: `rg "feature|syntax" .`.
2. Keep syntax close to nearby `.gp` files.
3. Run `goplus fmt file.gp`.
4. Run `goplus check path`.
5. For generated-Go or runtime issues, run
   `goplus transpile path --out-dir .goplusgen` and inspect
   `zz_goplus_gen.go`.
6. For executables, run `goplus run path --out-dir .goplusgen` or
   `goplus build path --out-dir .goplusgen`.

## Common Pitfalls

- Go `func` is now allowed for top-level GoPlus functions, as well as `fn`.
- Do not use `?` inside a function that does not return `!` or `T!`.
- Do not assume one `.gp` file equals one package in module mode; sibling files
  compile together.
- Do not access tagged enum payload internals directly; use `match`.
- Do not use `@memoize` on methods, error-returning functions, slices, maps, or
  function parameters.
- Do not trust undocumented syntax without compiling it; this language is new
  and intentionally smaller than Go.
