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

#[test]
fn emits_go_generic_syntax_for_tagged_enums() {
    let src = r#"
package main

enum Result<T> {
    Ok(T)
    Err(string)
}

fn load() -> Result<int> {
    return Result<int>::Ok(1)
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains("type Result[T any] struct"));
    assert!(go.contains("func ResultOk[T any](v0 T) Result[T]"));
    assert!(go.contains("return ResultOk[int](1)"));
    assert!(!go.contains("ResultOk<int>"));
    assert!(!go.contains("Result<int>"));
}

#[test]
fn emits_struct_tags_unchanged() {
    let src = r#"
package main

struct User {
    Email: string `json:"email" gorm:"column:email"`
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains(r#"Email string `json:"email" gorm:"column:email"`"#));
}

#[test]
fn lowers_try_on_chained_error_field() {
    let src = r#"
package main

fn seed(db: *DB) -> ! {
    db.Model().Where("key = ?", "course_level").Count().Error?
    return
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains(
        "if __gp_err0 := db.Model().Where(\"key = ?\", \"course_level\").Count().Error; __gp_err0 != nil {"
    ));
    assert!(!go.contains(".Error?"));
}

#[test]
fn returns_existing_error_var_in_type_with_error_functions() {
    let src = r#"
package main

fn load() -> *User! {
    err := fetchError()
    if err != nil {
        return err
    }
    return &User{}
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let model = analyze(&mut program).expect("sema ok");
    let go = generate_go(&program, &model);
    assert!(go.contains("return nil, err"));
    assert!(!go.contains("return err, nil"));
}
