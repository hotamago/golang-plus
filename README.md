# goplus

`goplus` is a Rust transpiler that compiles `*.gp` source files to Go.

## Features in v1

- Go-like syntax with `fn`
- Error sugar: `-> T!`, `-> !`, and `expr?`
- Simple enum + tagged enum (generic)
- Exhaustive `match` checking for enums
- `@derive(String)` for struct/enum
- Compile-time decorators for functions/methods:
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
