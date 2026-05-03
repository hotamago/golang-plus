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
    let imports = program
        .imports
        .iter()
        .map(|import| import.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(imports, vec!["fmt", "os"]);
}

#[test]
fn parse_import_aliases_and_raw_decls() {
    let src = r#"
package main

import (
    f "fmt"
    _ "embed"
    . "strings"
)

const answer = 42
var counter int
type Alias = string
"#;
    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.imports[0].alias.as_deref(), Some("f"));
    assert_eq!(program.imports[1].alias.as_deref(), Some("_"));
    assert_eq!(program.imports[2].alias.as_deref(), Some("."));
    assert_eq!(program.items.len(), 3);
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
    assert!(matches!(fn_decl.body.stmts[0], Stmt::Defer(_)));
    assert!(matches!(fn_decl.body.stmts[1], Stmt::Go(_)));
    assert!(matches!(fn_decl.body.stmts[2], Stmt::Assign(_)));
    assert!(matches!(fn_decl.body.stmts[4], Stmt::For(_)));
}

#[test]
fn parses_go_grouped_parameters() {
    let src = r#"
package main

func CheckPassword(password, hashed string) bool {
    return password == hashed
}

func GenerateToken(cfg *Config, id, passwordHash, tokenType string, expiresIn int) (string, error) {
    return "", nil
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert_eq!(fn_decl.params.len(), 2);
    assert_eq!(fn_decl.params[0].name, "password");
    assert_eq!(fn_decl.params[1].name, "hashed");
    assert_eq!(fn_decl.params[0].ty.raw, "string");
    assert_eq!(fn_decl.params[1].ty.raw, "string");
    let fn_decl = match &program.items[1] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert_eq!(
        fn_decl
            .params
            .iter()
            .map(|param| (param.name.as_str(), param.ty.raw.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("cfg", "*Config"),
            ("id", "string"),
            ("passwordHash", "string"),
            ("tokenType", "string"),
            ("expiresIn", "int")
        ]
    );
}

#[test]
fn parses_go_method_declaration_as_raw() {
    let src = r#"
package main

func (s *Server) routes(app *fiber.App) {
    app.Get("/", s.docs("ok"))
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    assert!(matches!(program.items[0], Item::Raw(_)));
}

#[test]
fn parses_comparison_in_if_condition() {
    let src = r#"
package main

func CheckPasswordStrength(password string) error {
    if len(password) < 8 {
        return nil
    }
    return nil
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert!(matches!(fn_decl.body.stmts[0], Stmt::If(_)));
}

#[test]
fn parses_c_style_for_with_index_assignment_as_raw_block() {
    let src = r#"
package main

fn randomText() -> string {
    alphabet := "abc"
    out := make([]byte, 10)
    for i := 0; i < len(out); i++ {
        out[i] = alphabet[i%len(alphabet)]
    }
    return string(out)
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert!(matches!(fn_decl.body.stmts[2], Stmt::For(_)));
}

#[test]
fn parses_struct_tags_and_raw_string_literals() {
    let src = r#"
package main

struct User {
    Email: string `json:"email" gorm:"column:email"`
}

fn pattern() -> string {
    return `[!@#$%^&*()]`
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let struct_decl = match &program.items[0] {
        Item::Struct(it) => it,
        _ => panic!("expected struct"),
    };
    assert_eq!(
        struct_decl.fields[0].tag.as_deref(),
        Some(r#"`json:"email" gorm:"column:email"`"#)
    );
}

#[test]
fn parses_password_validator_control_flow_as_raw_go() {
    let src = r#"
package auth

import "strings"
import "unicode"

fn CheckPasswordStrength(password: string) -> ! {
    upper := false
    lower := false
    digit := false
    special := false
    specialChars := "!@#$%^&*()_+-=[]{}|;:,.<>?"

    for _, r := range password {
        switch {
        case unicode.IsUpper(r):
            upper = true
        case unicode.IsLower(r):
            lower = true
        case unicode.IsDigit(r):
            digit = true
        case strings.ContainsRune(specialChars, r):
            special = true
        }
    }

    if !upper {
        return error("Password must contain at least one uppercase letter.")
    }
    return
}
"#;
    let program = parse_program(src).expect("parse should succeed");
    let fn_decl = match &program.items[0] {
        Item::Function(it) => it,
        _ => panic!("expected function"),
    };
    assert!(
        fn_decl
            .body
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, Stmt::For(_)))
    );
    assert!(
        fn_decl
            .body
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, Stmt::If(_)))
    );
}
