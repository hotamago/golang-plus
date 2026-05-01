use crate::{parser::parse_program, sema::analyze};

use super::generate_go;

#[test]
fn generates_retry_and_log_wrappers() {
    let src = r#"
package main

@log
@retry(3)
fn load(path: string) -> string! {
    value := read(path)?
    return value
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains("func load__inner"));
    assert!(go.contains("func load__decor0"));
    assert!(go.contains("func load("));
    assert!(go.contains("for attempt := 0; attempt < 3; attempt++"));
    assert!(go.contains("[goplus] enter load"));
}

#[test]
fn retry_without_backoff_does_not_import_time() {
    let src = r#"
package main

@retry(3)
fn load() -> string! {
    return "ok"
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(!go.contains("\"time\""));
}

#[test]
fn generates_custom_decorator_wrapper() {
    let src = r#"
package main

fn trace(next: func(path string) (string, error), label: string) -> func(path string) (string, error) {
    return next
}

@trace("io")
fn load(path: string) -> string! {
    return "ok"
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains("decorated := trace(load__inner, \"io\")"));
    assert!(go.contains("return decorated(path)"));
}

#[test]
fn normalizes_fn_style_types_and_literals() {
    let src = r#"
package main

fn trace(next: (fn(path: string) -> string!), label: string) -> (fn(path: string) -> string!) {
    wrapped := fn(path: string) -> string! {
        return next(path)
    }
    return wrapped
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains(
            "func trace(next (func(path string) (string, error)), label string) (func(path string) (string, error))"
        ));
    assert!(go.contains("wrapped := func(path string) (string, error)"));
}

#[test]
fn maps_main_error_wrapper() {
    let src = r#"
package main

fn main() -> ! {
    return
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains("func mainWarp() error"));
    assert!(go.contains("func main()"));
}

#[test]
fn generates_memoize_cache() {
    let src = r#"
package main

@memoize
fn add(a: int, b: int) -> int {
    return a + b
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains("type add__decor0Key struct"));
    assert!(go.contains("var add__decor0Cache"));
    assert!(go.contains("sync.Mutex"));
}
