use crate::{parser::parse_program, sema::analyze};

#[test]
fn allows_custom_decorator() {
    let src = r#"
package main

fn foo(next: func() string) -> func() string {
    return next
}

@foo
fn run() -> string {
    return "ok"
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    result.expect("sema should succeed");
}

#[test]
fn enforces_retry_on_error_functions() {
    let src = r#"
package main

@retry(3)
fn run() -> string {
    return "ok"
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    assert!(result.is_err());
}

#[test]
fn catches_non_exhaustive_match() {
    let src = r#"
package main

enum Status {
    Pending
    Running
    Done
}

fn label(s: Status) -> string {
    match s {
        Status::Pending => "pending",
        Status::Running => "running",
    }
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    assert!(result.is_err());
}

#[test]
fn rejects_memoize_with_slice_param() {
    let src = r#"
package main

@memoize
fn sum(items: []int) -> int {
    return 1
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    assert!(result.is_err());
}

#[test]
fn rejects_memoize_on_methods() {
    let src = r#"
package main

struct User {
    name: string
}

impl User {
    @memoize
    fn greet(self) -> string {
        return "hi"
    }
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    assert!(result.is_err());
}

#[test]
fn rejects_duplicate_declarations_and_reserved_generated_names() {
    let src = r#"
package main

fn load() {}
fn load() {}
fn mainWarp() {}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    let diagnostics = result.expect_err("sema should fail");
    assert!(diagnostics.iter().any(|diag| diag.code == "E0100"));
    assert!(diagnostics.iter().any(|diag| diag.code == "E0101"));
}

#[test]
fn rejects_tagged_match_payload_arity_and_duplicate_arms() {
    let src = r#"
package main

enum Result<T> {
    Ok(T)
    Err(string)
}

fn label(r: Result<int>) -> string {
    match r {
        Ok => "ok",
        Ok(v) => "again",
        Err(e) => e,
    }
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    let diagnostics = result.expect_err("sema should fail");
    assert!(diagnostics.iter().any(|diag| diag.code == "E0200"));
    assert!(diagnostics.iter().any(|diag| diag.code == "E0201"));
}

#[test]
fn validates_known_custom_decorator_argument_count() {
    let src = r#"
package main

fn trace(next: func() string, label: string) -> func() string {
    return next
}

@trace
fn run() -> string {
    return "ok"
}
"#;
    let mut program = parse_program(src).expect("parse ok");
    let result = analyze(&mut program);
    let diagnostics = result.expect_err("sema should fail");
    assert!(diagnostics.iter().any(|diag| diag.code == "E0302"));
}
