# goplus

`goplus` is a Rust transpiler that compiles `*.gp` source files to Go.

## Features in v1

- Go-like syntax with `fn`
- Error sugar: `-> T!`, `-> !`, and `expr?`
- Simple enum + tagged enum (generic)
- Exhaustive `match` checking for enums
- `@derive(String)` for struct/enum
- Compile-time decorators for functions/methods:
  - Custom decorators (Python-like factory style)
  - `@log`
  - `@retry(times[, backoff_ms])`
  - `@memoize` (top-level non-error functions)
- CLI:
  - `goplus check`
  - `goplus transpile`
  - `goplus build`
  - `goplus run`

## Quick Start

```bash
cargo run -- transpile examples/demo.gp --out-dir .goplusgen
cargo run -- run examples/demo.gp --out-dir .goplusgen
```

## Example

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

`goplus` now supports user-defined decorators. A custom decorator is a function that receives
`next` (the previous function in the chain) plus optional decorator args, and returns a function
with the same signature.

```gp
package main

import "fmt"

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
