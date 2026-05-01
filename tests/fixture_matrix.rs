use std::{fs, path::PathBuf};

use goplus::{
    ast::{Item, Stmt},
    codegen::generate_go,
    compiler::{
        DiagnosticFormat, build_file, check_file_with_format, transpile_file,
        transpile_file_with_options,
    },
    parser::parse_program,
    sema::analyze,
};
use tempfile::tempdir;

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(path)
}

#[test]
fn parser_fixture_covers_aliases_raw_decls_and_structured_raw_statements() {
    let source = fs::read_to_string(fixture("parser/aliases_and_raw.gp")).expect("fixture");
    let program = parse_program(&source).expect("parse fixture");
    assert_eq!(program.imports[0].alias.as_deref(), Some("f"));
    assert_eq!(program.imports[1].alias.as_deref(), Some("_"));
    assert_eq!(program.imports[2].alias.as_deref(), Some("."));
    assert!(matches!(program.items[0], Item::Raw(_)));
    let function = match program.items.last().expect("function") {
        Item::Function(function) => function,
        _ => panic!("expected function"),
    };
    assert!(matches!(function.body.stmts[0], Stmt::Defer(_)));
    assert!(matches!(function.body.stmts[1], Stmt::Go(_)));
    assert!(matches!(function.body.stmts[2], Stmt::Assign(_)));
}

#[test]
fn codegen_fixture_uses_go_generic_syntax_for_tagged_enums() {
    let source = fs::read_to_string(fixture("codegen/generic_tagged.gp")).expect("fixture");
    let mut program = parse_program(&source).expect("parse fixture");
    let model = analyze(&mut program).expect("analyze fixture");
    let go = generate_go(&program, &model);
    assert!(go.contains("type Result[T any] struct"));
    assert!(go.contains("func ResultOk[T any](v0 T) Result[T]"));
    assert!(go.contains("return ResultOk[int](7)"));
    assert!(!go.contains("Result<int>"));
}

#[test]
fn codegen_fixture_matches_golden_go() {
    let out_dir = tempdir().expect("tempdir");
    transpile_file(&fixture("codegen/generic_tagged.gp"), out_dir.path()).expect("transpile");
    let generated = fs::read_to_string(out_dir.path().join("zz_goplus_gen.go")).expect("generated");
    let golden = fs::read_to_string(fixture("codegen/generic_tagged.golden.go")).expect("golden");
    assert_eq!(normalize_newlines(&generated), normalize_newlines(&golden));
}

#[test]
fn diagnostics_fixture_reports_codes_spans_and_hints() {
    let err = check_file_with_format(&fixture("diagnostics/bad_retry.gp"), DiagnosticFormat::Json)
        .expect_err("fixture should fail");
    let rendered = err.to_string();
    assert!(rendered.contains(r#""code":"E0303""#));
    assert!(rendered.contains(r#""code":"E0305""#));
    assert!(rendered.contains(r#""span""#));
    assert!(rendered.contains(r#""hint""#));
}

#[test]
fn source_map_fixture_emits_real_mappings() {
    let out_dir = tempdir().expect("tempdir");
    transpile_file_with_options(&fixture("source_maps/mapped.gp"), out_dir.path(), true)
        .expect("transpile");
    let map = fs::read_to_string(out_dir.path().join("zz_goplus_gen.go.map")).expect("map");
    assert!(map.contains(r#""kind": "function""#));
    assert!(map.contains(r#""kind": "match_arm""#));
    assert!(map.contains(r#""source_start""#));
    assert!(!map.contains(r#""mappings": []"#));
}

#[test]
fn build_fixture_compiles_aliases_raw_decls_and_generic_tagged_enums() {
    let out_dir = tempdir().expect("out");
    let bin_dir = tempdir().expect("bin");
    let out_bin = bin_dir
        .path()
        .join(if cfg!(windows) { "app.exe" } else { "app" });
    build_file(
        &fixture("build/generic_app.gp"),
        out_dir.path(),
        Some(&out_bin),
    )
    .expect("build fixture");
    let generated = fs::read_to_string(out_dir.path().join("zz_goplus_gen.go")).expect("generated");
    assert!(generated.contains("f \"fmt\""));
    assert!(generated.contains("ResultOk[int](7)"));
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}
