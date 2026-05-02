use std::collections::HashSet;

use crate::ast::*;
use crate::diag::{Diagnostic, DiagnosticSeverity};
use crate::sema::SemanticModel;

/// Run lint rules on a parsed and analyzed program.
/// Returns a list of warning-level diagnostics.
pub fn lint_program(program: &Program, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    lint_unused_imports(program, &mut diagnostics);
    lint_naming_conventions(program, &mut diagnostics);
    lint_empty_function_bodies(program, &mut diagnostics);
    lint_redundant_return(program, &mut diagnostics);
    lint_large_functions(program, &mut diagnostics);
    lint_missing_string_derive(program, model, &mut diagnostics);

    diagnostics
}

/// L0001: Unused imports — imported but never referenced in source items.
fn lint_unused_imports(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let all_text = collect_all_text(program);

    for import in &program.imports {
        let binding = if let Some(alias) = &import.alias {
            if alias == "_" || alias == "." {
                continue;
            }
            alias.clone()
        } else {
            import
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&import.path)
                .to_string()
        };

        if !all_text.contains(&binding) {
            diagnostics.push(
                Diagnostic::warning(
                    format!("import `{}` appears unused", import.path),
                    Some(import.span.clone()),
                )
                .with_code("L0001")
                .with_hint("remove the import or use it in the code")
                .with_severity(DiagnosticSeverity::Warning),
            );
        }
    }
}

/// Collect all identifiers / text from the program items for import usage checking.
fn collect_all_text(program: &Program) -> HashSet<String> {
    let mut texts = HashSet::new();
    for item in &program.items {
        match item {
            Item::Function(f) => {
                collect_block_idents(&f.body, &mut texts);
                for p in &f.params {
                    texts.insert(p.ty.raw.clone());
                }
                if let Some(ty) = f.ret.value_type() {
                    texts.insert(ty.raw.clone());
                }
            }
            Item::Impl(impl_block) => {
                texts.insert(impl_block.target.clone());
                for method in &impl_block.methods {
                    collect_block_idents(&method.body, &mut texts);
                    for p in &method.params {
                        texts.insert(p.ty.raw.clone());
                    }
                    if let Some(ty) = method.ret.value_type() {
                        texts.insert(ty.raw.clone());
                    }
                }
            }
            Item::Struct(s) => {
                for field in &s.fields {
                    texts.insert(field.ty.raw.clone());
                }
            }
            Item::Enum(e) => {
                for variant in &e.variants {
                    for payload in &variant.payload {
                        texts.insert(payload.raw.clone());
                    }
                }
            }
            Item::Raw(raw) => {
                for word in raw.text.split_whitespace() {
                    texts.insert(word.to_string());
                }
            }
        }
    }
    texts
}

fn collect_block_idents(block: &Block, texts: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::VarDecl(v) => {
                texts.insert(v.expr.text.clone());
            }
            Stmt::Assign(a) => {
                texts.insert(a.text.clone());
            }
            Stmt::Return(r) => {
                for expr in &r.exprs {
                    texts.insert(expr.text.clone());
                }
            }
            Stmt::Expr(e) => {
                texts.insert(e.expr.text.clone());
            }
            Stmt::Match(m) => {
                texts.insert(m.value.text.clone());
                for arm in &m.arms {
                    match &arm.body {
                        MatchArmBody::Expr(expr) => {
                            texts.insert(expr.text.clone());
                        }
                        MatchArmBody::Block(block) => collect_block_idents(block, texts),
                    }
                }
            }
            Stmt::If(i) => {
                texts.insert(i.condition.text.clone());
                collect_block_idents(&i.then_block, texts);
                if let Some(else_branch) = &i.else_branch {
                    match else_branch {
                        ElseBranch::Block(block) => collect_block_idents(block, texts),
                        ElseBranch::If(nested) => {
                            collect_block_idents(
                                &Block {
                                    stmts: vec![Stmt::If((**nested).clone())],
                                    span: nested.span.clone(),
                                },
                                texts,
                            );
                        }
                    }
                }
            }
            Stmt::Defer(raw)
            | Stmt::Go(raw)
            | Stmt::For(raw)
            | Stmt::Switch(raw)
            | Stmt::Select(raw)
            | Stmt::Raw(raw) => {
                for word in raw.text.split_whitespace() {
                    texts.insert(word.to_string());
                }
            }
        }
    }
}

/// L0002: Naming conventions — structs/enums should be PascalCase, fn names camelCase.
fn lint_naming_conventions(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    for item in &program.items {
        match item {
            Item::Struct(s) => {
                if !s.name.is_empty() && !s.name.chars().next().unwrap().is_uppercase() {
                    diagnostics.push(
                        Diagnostic::warning(
                            format!("struct `{}` should use PascalCase naming", s.name),
                            Some(s.span.clone()),
                        )
                        .with_code("L0002")
                        .with_severity(DiagnosticSeverity::Warning),
                    );
                }
            }
            Item::Enum(e) => {
                if !e.name.is_empty() && !e.name.chars().next().unwrap().is_uppercase() {
                    diagnostics.push(
                        Diagnostic::warning(
                            format!("enum `{}` should use PascalCase naming", e.name),
                            Some(e.span.clone()),
                        )
                        .with_code("L0002")
                        .with_severity(DiagnosticSeverity::Warning),
                    );
                }
            }
            _ => {}
        }
    }
}

/// L0003: Empty function body.
fn lint_empty_function_bodies(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    for item in &program.items {
        if let Item::Function(f) = item {
            if f.body.stmts.is_empty() && f.name != "main" {
                diagnostics.push(
                    Diagnostic::warning(
                        format!("function `{}` has an empty body", f.name),
                        Some(f.span.clone()),
                    )
                    .with_code("L0003")
                    .with_hint("add an implementation or remove the function")
                    .with_severity(DiagnosticSeverity::Warning),
                );
            }
        }
    }
}

/// L0004: Redundant return at end of void function.
fn lint_redundant_return(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    for item in &program.items {
        if let Item::Function(f) = item {
            if matches!(f.ret, ReturnType::Void) {
                if let Some(Stmt::Return(ret)) = f.body.stmts.last() {
                    if ret.exprs.is_empty() {
                        diagnostics.push(
                            Diagnostic::warning(
                                "redundant `return` at end of void function",
                                Some(ret.span.clone()),
                            )
                            .with_code("L0004")
                            .with_hint("remove the trailing `return`")
                            .with_severity(DiagnosticSeverity::Warning),
                        );
                    }
                }
            }
        }
    }
}

/// L0006: Large function body (> 50 statements).
fn lint_large_functions(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    for item in &program.items {
        if let Item::Function(f) = item {
            let count = count_stmts_recursive(&f.body);
            if count > 50 {
                diagnostics.push(
                    Diagnostic::warning(
                        format!(
                            "function `{}` has {} statements; consider refactoring",
                            f.name, count
                        ),
                        Some(f.span.clone()),
                    )
                    .with_code("L0006")
                    .with_hint("break large functions into smaller helpers")
                    .with_severity(DiagnosticSeverity::Warning),
                );
            }
        }
    }
}

fn count_stmts_recursive(block: &Block) -> usize {
    let mut count = block.stmts.len();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Match(m) => {
                for arm in &m.arms {
                    if let MatchArmBody::Block(b) = &arm.body {
                        count += count_stmts_recursive(b);
                    }
                }
            }
            Stmt::If(i) => {
                count += count_stmts_recursive(&i.then_block);
                if let Some(ElseBranch::Block(b)) = &i.else_branch {
                    count += count_stmts_recursive(b);
                }
            }
            _ => {}
        }
    }
    count
}

/// L0007: Missing `@derive(String)` on enums used in format expressions.
fn lint_missing_string_derive(
    program: &Program,
    _model: &SemanticModel,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let all_text = collect_all_text(program);

    for item in &program.items {
        if let Item::Enum(e) = item {
            let has_string_derive = e.derives.iter().any(|d| matches!(d, DeriveKind::String));
            if !has_string_derive {
                // Check if enum is used in format-like expressions
                let enum_used_in_format = all_text.iter().any(|text| {
                    (text.contains("Println") || text.contains("Printf") || text.contains("Sprintf"))
                        && text.contains(&e.name)
                });
                if enum_used_in_format {
                    diagnostics.push(
                        Diagnostic::warning(
                            format!(
                                "enum `{}` is used in format expressions but has no `@derive(String)`",
                                e.name
                            ),
                            Some(e.span.clone()),
                        )
                        .with_code("L0007")
                        .with_hint("add `@derive(String)` to generate a human-readable String() method")
                        .with_severity(DiagnosticSeverity::Warning),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;
    use crate::sema;

    fn lint_source(source: &str) -> Vec<Diagnostic> {
        let mut program = parse_program(source).expect("parse");
        let model = sema::analyze(&mut program).expect("analyze");
        lint_program(&program, &model)
    }

    #[test]
    fn lint_detects_empty_function() {
        let warnings = lint_source("package main\n\nfn helper() {\n}\n");
        assert!(warnings.iter().any(|d| d.code == "L0003"));
    }

    #[test]
    fn lint_detects_redundant_return() {
        let warnings = lint_source("package main\n\nfn helper() {\n\treturn\n}\n");
        assert!(warnings.iter().any(|d| d.code == "L0004"));
    }

    #[test]
    fn lint_no_warning_for_non_void_return() {
        let warnings = lint_source("package main\n\nfn helper() -> int {\n\treturn 42\n}\n");
        assert!(warnings.iter().all(|d| d.code != "L0004"));
    }

    #[test]
    fn lint_all_warnings_are_warning_severity() {
        let warnings = lint_source("package main\n\nfn helper() {\n}\n");
        for w in &warnings {
            assert_eq!(w.severity, DiagnosticSeverity::Warning);
        }
    }
}
