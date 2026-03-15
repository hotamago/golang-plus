# link-source

This example demonstrates package-level compilation with mostly GoPlus sources inside one real Go module.

- `go.mod` defines the module root.
- `main.gp` and `messages.gp` are GoPlus files in the same `package main`.
- `bridge.go` is the only native Go file here, kept on purpose to prove GoPlus <-> Go linking still works.
- `pkg/support/support.gp` is a separate imported GoPlus package.
- `internal/label/label.gp` is another nested imported GoPlus package.

Why can `main.gp`, `messages.gp`, and `bridge.go` call each other without `import`?

- Because they are all part of the same Go package: `package main`.
- In Go and GoPlus, files inside the same package share one namespace.
- `import` is only required when crossing a package boundary, such as importing `example.com/goplus-link-source/pkg/support`.

Run it from the repository root:

```bash
cargo run -- run examples/link-source/main.gp --out-dir .goplusgen
```
