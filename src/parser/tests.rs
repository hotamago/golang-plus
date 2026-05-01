use super::parse_program;
use crate::ast::{Item, Pattern, ReturnType, Stmt};

#[test]
fn parse_decorated_function() {
    let src = r#"
package main

@log
@retry(3, 100)
fn load(path: string) -> string! {
    value := read(path)?
    return value
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert_eq!(fn_decl.decorators.len(), 2);
    assert!(matches!(fn_decl.ret, ReturnType::TypeWithError(_)));
    match &fn_decl.body.stmts[0] {
        Stmt::VarDecl(stmt) => assert!(stmt.expr.has_try),
        _ => panic!("expected var decl"),
    }
}

#[test]
fn parse_match_arm_patterns() {
    let src = r#"
package main

enum Result<T, E> {
    Ok(T)
    Err(E)
}

fn show(r: Result<int, string>) -> string {
    match r {
        Ok(v) => "ok",
        Err(e) => "err",
    }
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[1] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    let match_stmt = match &fn_decl.body.stmts[0] {
        Stmt::Match(stmt) => stmt,
        _ => panic!("expected match"),
    };
    assert!(matches!(
        match_stmt.arms[0].pattern,
        Pattern::Variant { .. }
    ));
}

#[test]
fn parse_qualified_decorator_name() {
    let src = r#"
package main

@decorators.trace("svc")
fn run() {
    return
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert_eq!(fn_decl.decorators[0].name, "decorators.trace");
}

#[test]
fn parse_grouped_imports_and_block_comments() {
    let src = r#"
package main

/*
multi-line package note
*/
import (
    "fmt"
    "os"
)

fn main() -> ! {
    fmt.Println(os.Args)
    return
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.imports, vec!["fmt", "os"]);
}

#[test]
fn parses_common_raw_go_statements() {
    let src = r#"
package main

fn main() {
    defer cleanup()
    go worker()
    i = i + 1
    i++
    for i < 10 {
        i++
    }
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert_eq!(fn_decl.body.stmts.len(), 5);
}
