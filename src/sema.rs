use std::collections::{BTreeSet, HashMap};

use crate::{ast::*, diag::Diagnostic};

#[derive(Debug, Clone, Default)]
pub struct SemanticModel {
    pub enums: HashMap<String, EnumDecl>,
    pub function_returns: HashMap<String, ReturnType>,
}

pub fn analyze(program: &mut Program) -> Result<SemanticModel, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut model = SemanticModel::default();

    for item in &program.items {
        match item {
            Item::Enum(enum_decl) => {
                model
                    .enums
                    .insert(enum_decl.name.clone(), enum_decl.clone());
            }
            Item::Function(fn_decl) => {
                model
                    .function_returns
                    .insert(fn_decl.name.clone(), fn_decl.ret.clone());
            }
            Item::Impl(impl_block) => {
                for method in &impl_block.methods {
                    model.function_returns.insert(
                        format!("{}.{}", base_enum_name(&impl_block.target), method.name),
                        method.ret.clone(),
                    );
                }
            }
            Item::Struct(_) => {}
        }
    }

    for item in &mut program.items {
        match item {
            Item::Function(function) => {
                analyze_function(function, false, &model.enums, &mut diagnostics);
            }
            Item::Impl(impl_block) => {
                for method in &mut impl_block.methods {
                    analyze_method(method, &impl_block.target, &model.enums, &mut diagnostics);
                }
            }
            Item::Struct(_) | Item::Enum(_) => {}
        }
    }

    if diagnostics.is_empty() {
        Ok(model)
    } else {
        Err(diagnostics)
    }
}

fn analyze_function(
    function: &mut FnDecl,
    is_method: bool,
    enums: &HashMap<String, EnumDecl>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_decorators(
        &function.decorators,
        &function.ret,
        &function.params,
        is_method,
        diagnostics,
    );

    let mut vars = HashMap::new();
    analyze_block(
        &mut function.body,
        &function.ret,
        enums,
        diagnostics,
        &mut vars,
    );
}

fn analyze_method(
    method: &mut MethodDecl,
    target: &str,
    enums: &HashMap<String, EnumDecl>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_decorators(
        &method.decorators,
        &method.ret,
        &method.params,
        true,
        diagnostics,
    );

    let mut vars = HashMap::new();
    vars.insert("self".to_string(), base_enum_name(target).to_string());
    analyze_block(&mut method.body, &method.ret, enums, diagnostics, &mut vars);
}

fn analyze_block(
    block: &mut Block,
    ret_type: &ReturnType,
    enums: &HashMap<String, EnumDecl>,
    diagnostics: &mut Vec<Diagnostic>,
    vars: &mut HashMap<String, String>,
) {
    for stmt in &mut block.stmts {
        match stmt {
            Stmt::VarDecl(var_decl) => {
                validate_try_usage(&var_decl.expr, ret_type, diagnostics);
                if let Some(enum_name) = infer_enum_type_from_expr(&var_decl.expr.text, enums) {
                    vars.insert(var_decl.name.clone(), enum_name);
                }
            }
            Stmt::Return(ret) => {
                for expr in &ret.exprs {
                    validate_try_usage(expr, ret_type, diagnostics);
                }
            }
            Stmt::Expr(expr_stmt) => {
                validate_try_usage(&expr_stmt.expr, ret_type, diagnostics);
            }
            Stmt::Match(match_stmt) => {
                validate_try_usage(&match_stmt.value, ret_type, diagnostics);
                analyze_match_stmt(match_stmt, ret_type, enums, diagnostics, vars);
            }
            Stmt::If(if_stmt) => {
                validate_try_usage(&if_stmt.condition, ret_type, diagnostics);
                let mut then_vars = vars.clone();
                analyze_block(
                    &mut if_stmt.then_block,
                    ret_type,
                    enums,
                    diagnostics,
                    &mut then_vars,
                );
                if let Some(else_branch) = &mut if_stmt.else_branch {
                    match else_branch {
                        ElseBranch::Block(block) => {
                            let mut else_vars = vars.clone();
                            analyze_block(block, ret_type, enums, diagnostics, &mut else_vars);
                        }
                        ElseBranch::If(nested_if) => {
                            let mut else_vars = vars.clone();
                            analyze_block(
                                &mut Block {
                                    stmts: vec![Stmt::If((**nested_if).clone())],
                                    span: nested_if.span.clone(),
                                },
                                ret_type,
                                enums,
                                diagnostics,
                                &mut else_vars,
                            );
                        }
                    }
                }
            }
            Stmt::Raw(_) => {}
        }
    }
}

fn analyze_match_stmt(
    match_stmt: &mut MatchStmt,
    ret_type: &ReturnType,
    enums: &HashMap<String, EnumDecl>,
    diagnostics: &mut Vec<Diagnostic>,
    vars: &HashMap<String, String>,
) {
    let wildcard_exists = match_stmt
        .arms
        .iter()
        .any(|arm| matches!(arm.pattern, Pattern::Wildcard { .. }));

    let mut resolved_enum = match_stmt.arms.iter().find_map(|arm| match &arm.pattern {
        Pattern::TypedVariant { enum_name, .. } => Some(base_enum_name(enum_name).to_string()),
        _ => None,
    });

    if resolved_enum.is_none() {
        if let Some(found) = vars.get(match_stmt.value.text.trim()) {
            resolved_enum = Some(found.clone());
        }
    }

    if resolved_enum.is_none() {
        let variant_names = match_stmt
            .arms
            .iter()
            .filter_map(|arm| arm.pattern.variant_name().map(str::to_string))
            .collect::<Vec<_>>();
        let candidates = enums
            .values()
            .filter(|enum_decl| {
                let enum_variants = enum_decl
                    .variants
                    .iter()
                    .map(|v| v.name.clone())
                    .collect::<BTreeSet<_>>();
                variant_names
                    .iter()
                    .all(|name| enum_variants.contains(name))
            })
            .map(|enum_decl| enum_decl.name.clone())
            .collect::<Vec<_>>();
        if candidates.len() == 1 {
            resolved_enum = Some(candidates[0].clone());
        }
    }

    if let Some(enum_name) = resolved_enum {
        match_stmt.resolved_enum = Some(enum_name.clone());
        if let Some(enum_decl) = enums.get(&enum_name) {
            let all_variants = enum_decl
                .variants
                .iter()
                .map(|v| v.name.clone())
                .collect::<BTreeSet<_>>();
            let mut seen = BTreeSet::new();
            for arm in &mut match_stmt.arms {
                match &arm.pattern {
                    Pattern::Wildcard { .. } => {}
                    Pattern::TypedVariant {
                        enum_name, variant, ..
                    } => {
                        if base_enum_name(enum_name) != enum_decl.name {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "match arm uses enum `{}` but expected `{}`",
                                    base_enum_name(enum_name),
                                    enum_decl.name
                                ),
                                Some(arm.span.clone()),
                            ));
                        }
                        if !all_variants.contains(variant) {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "unknown variant `{variant}` for enum `{}`",
                                    enum_decl.name
                                ),
                                Some(arm.span.clone()),
                            ));
                        }
                        seen.insert(variant.clone());
                    }
                    Pattern::Variant { variant, .. } => {
                        if !all_variants.contains(variant) {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "unknown variant `{variant}` for enum `{}`",
                                    enum_decl.name
                                ),
                                Some(arm.span.clone()),
                            ));
                        }
                        seen.insert(variant.clone());
                    }
                }

                match &mut arm.body {
                    MatchArmBody::Expr(expr) => validate_try_usage(expr, ret_type, diagnostics),
                    MatchArmBody::Block(block) => {
                        let mut scoped_vars = HashMap::new();
                        analyze_block(block, ret_type, enums, diagnostics, &mut scoped_vars);
                    }
                }
            }

            if !wildcard_exists && seen != all_variants {
                let missing = all_variants
                    .difference(&seen)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(
                    Diagnostic::new(
                        format!("non-exhaustive match for enum `{}`", enum_decl.name),
                        Some(match_stmt.span.clone()),
                    )
                    .with_hint(format!("missing variants: {missing}")),
                );
            }
        }
    } else if !wildcard_exists {
        diagnostics.push(
            Diagnostic::new(
                "cannot resolve enum type for match expression",
                Some(match_stmt.span.clone()),
            )
            .with_hint("add explicit typed patterns, e.g. `Status::Pending`"),
        );
    }
}

fn validate_try_usage(expr: &Expr, ret_type: &ReturnType, diagnostics: &mut Vec<Diagnostic>) {
    if expr.has_try && !ret_type.is_error_capable() {
        diagnostics.push(
            Diagnostic::new(
                "`?` can only be used in functions returning `!` or `T!`",
                Some(expr.span.clone()),
            )
            .with_hint("change function return type to `!` or `T!`"),
        );
    }
}

fn validate_decorators(
    decorators: &[Decorator],
    ret_type: &ReturnType,
    params: &[ParamDecl],
    is_method: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for decorator in decorators {
        match decorator.name.as_str() {
            "log" => {}
            "retry" => {
                if !ret_type.is_error_capable() {
                    diagnostics.push(Diagnostic::new(
                        "`@retry` requires function returning `!` or `T!`",
                        Some(decorator.span.clone()),
                    ));
                }
                if decorator.args.is_empty() || decorator.args.len() > 2 {
                    diagnostics.push(
                        Diagnostic::new(
                            "`@retry` expects `@retry(times)` or `@retry(times, backoff_ms)`",
                            Some(decorator.span.clone()),
                        )
                        .with_hint("example: @retry(3, 100)"),
                    );
                }
                for arg in &decorator.args {
                    if parse_positive_int(arg).is_none() {
                        diagnostics.push(Diagnostic::new(
                            format!("`@retry` argument `{arg}` must be a positive integer"),
                            Some(decorator.span.clone()),
                        ));
                    }
                }
            }
            "memoize" => {
                if is_method {
                    diagnostics.push(Diagnostic::new(
                        "`@memoize` is allowed only on top-level functions",
                        Some(decorator.span.clone()),
                    ));
                }
                if !matches!(ret_type, ReturnType::Type(_)) {
                    diagnostics.push(Diagnostic::new(
                        "`@memoize` requires non-error return type `T`",
                        Some(decorator.span.clone()),
                    ));
                }
                for param in params {
                    if !is_comparable_type(&param.ty.raw) {
                        diagnostics.push(
                            Diagnostic::new(
                                format!(
                                    "`@memoize` parameter `{}` with type `{}` is not comparable",
                                    param.name, param.ty.raw
                                ),
                                Some(param.span.clone()),
                            )
                            .with_hint("use scalar/pointer/named comparable types only"),
                        );
                    }
                }
            }
            _ => {
                // Custom decorators are allowed.
                // They are emitted as decorator-factory wrappers in Go codegen.
            }
        }
    }
}

fn parse_positive_int(text: &str) -> Option<u64> {
    let cleaned = text.trim().replace('_', "");
    let value = cleaned.parse::<u64>().ok()?;
    if value == 0 { None } else { Some(value) }
}

fn is_comparable_type(ty: &str) -> bool {
    let trimmed = ty.trim();
    !trimmed.starts_with("[]")
        && !trimmed.starts_with("map[")
        && !trimmed.starts_with("func(")
        && !trimmed.contains("[]")
}

fn infer_enum_type_from_expr(expr: &str, enums: &HashMap<String, EnumDecl>) -> Option<String> {
    let (left, _) = expr.split_once("::")?;
    let candidate = base_enum_name(left.trim()).to_string();
    if enums.contains_key(&candidate) {
        Some(candidate)
    } else {
        None
    }
}

pub fn base_enum_name(input: &str) -> &str {
    input.split('<').next().unwrap_or(input).trim()
}

#[cfg(test)]
mod tests {
    use crate::{parser::parse_program, sema::analyze};

    #[test]
    fn allows_custom_decorator() {
        let src = r#"
package main

@foo
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
        assert!(result.is_ok());
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
}
