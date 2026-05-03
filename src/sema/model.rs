use super::*;

#[derive(Debug, Clone, Default)]
pub struct SemanticModel {
    pub enums: HashMap<String, EnumDecl>,
    pub function_returns: HashMap<String, ReturnType>,
    pub function_params: HashMap<String, Vec<ParamDecl>>,
}

pub fn analyze(program: &mut Program) -> Result<SemanticModel, Vec<Diagnostic>> {
    let model = build_model(std::iter::once(&*program));
    analyze_with_model(program, &model)?;
    Ok(model)
}

pub fn build_model<'a>(programs: impl IntoIterator<Item = &'a Program>) -> SemanticModel {
    let mut model = SemanticModel::default();

    for program in programs {
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
                    model
                        .function_params
                        .insert(fn_decl.name.clone(), fn_decl.params.clone());
                }
                Item::Impl(impl_block) => {
                    for method in &impl_block.methods {
                        model.function_returns.insert(
                            format!("{}.{}", base_enum_name(&impl_block.target), method.name),
                            method.ret.clone(),
                        );
                    }
                }
                Item::Struct(_) | Item::Raw(_) => {}
            }
        }
    }

    model
}

pub fn analyze_with_model(
    program: &mut Program,
    model: &SemanticModel,
) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    validate_declarations(program, &mut diagnostics);

    for item in &mut program.items {
        match item {
            Item::Function(function) => {
                analyze_function(function, false, model, &mut diagnostics);
            }
            Item::Impl(impl_block) => {
                for method in &mut impl_block.methods {
                    analyze_method(method, &impl_block.target, &model.enums, &mut diagnostics);
                }
            }
            Item::Struct(_) | Item::Enum(_) | Item::Raw(_) => {}
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn validate_declarations(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashMap::<String, Span>::new();
    for item in &program.items {
        let (kind, name, span) = match item {
            Item::Struct(decl) => ("struct", &decl.name, decl.span.clone()),
            Item::Enum(decl) => ("enum", &decl.name, decl.span.clone()),
            Item::Function(decl) => ("function", &decl.name, decl.span.clone()),
            Item::Impl(_) | Item::Raw(_) => continue,
        };

        if let Some(first_span) = names.insert(name.clone(), span.clone()) {
            diagnostics.push(
                Diagnostic::new(
                    format!("duplicate declaration `{name}`"),
                    Some(span.clone()),
                )
                .with_code("E0100")
                .with_hint(format!(
                    "the first declaration starts at byte {}",
                    first_span.start
                )),
            );
        }

        if name == "mainWarp" || name.starts_with("__goplus") || name.contains("__decor") {
            diagnostics.push(
                Diagnostic::new(
                    format!("{kind} `{name}` collides with generated GoPlus symbols"),
                    Some(span),
                )
                .with_code("E0101")
                .with_hint("choose a name without GoPlus reserved generated suffixes"),
            );
        }

        if let Item::Enum(enum_decl) = item {
            validate_enum_generated_names(enum_decl, diagnostics);
        }
    }
}

fn validate_enum_generated_names(enum_decl: &EnumDecl, diagnostics: &mut Vec<Diagnostic>) {
    let mut variants = HashMap::<String, Span>::new();
    for variant in &enum_decl.variants {
        if let Some(first_span) = variants.insert(variant.name.clone(), variant.span.clone()) {
            diagnostics.push(
                Diagnostic::new(
                    format!("duplicate enum variant `{}`", variant.name),
                    Some(variant.span.clone()),
                )
                .with_code("E0102")
                .with_hint(format!(
                    "the first variant starts at byte {}",
                    first_span.start
                )),
            );
        }

        let generated_ctor = format!("{}{}", enum_decl.name, variant.name);
        if generated_ctor == "mainWarp"
            || generated_ctor.starts_with("__goplus")
            || generated_ctor.contains("__decor")
        {
            diagnostics.push(
                Diagnostic::new(
                    format!(
                        "enum variant `{}` creates generated symbol `{generated_ctor}` that collides with GoPlus internals",
                        variant.name
                    ),
                    Some(variant.span.clone()),
                )
                .with_code("E0101")
                .with_hint("rename the enum or variant to avoid reserved generated symbols"),
            );
        }
    }
}

pub(super) fn analyze_function(
    function: &mut FnDecl,
    is_method: bool,
    model: &SemanticModel,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_decorators(
        &function.decorators,
        &function.ret,
        &function.params,
        is_method,
        &model.function_params,
        diagnostics,
    );

    let mut vars = HashMap::new();
    for param in &function.params {
        if model.enums.contains_key(base_enum_name(&param.ty.raw)) {
            vars.insert(
                param.name.clone(),
                base_enum_name(&param.ty.raw).to_string(),
            );
        }
    }
    analyze_block(
        &mut function.body,
        &function.ret,
        &model.enums,
        diagnostics,
        &mut vars,
    );
}

pub(super) fn analyze_method(
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
        &HashMap::new(),
        diagnostics,
    );

    let mut vars = HashMap::new();
    vars.insert("self".to_string(), base_enum_name(target).to_string());
    for param in &method.params {
        if enums.contains_key(base_enum_name(&param.ty.raw)) {
            vars.insert(
                param.name.clone(),
                base_enum_name(&param.ty.raw).to_string(),
            );
        }
    }
    analyze_block(&mut method.body, &method.ret, enums, diagnostics, &mut vars);
}

pub(super) fn analyze_block(
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
            Stmt::Assign(_) => {}
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
            Stmt::Defer(_)
            | Stmt::Go(_)
            | Stmt::For(_)
            | Stmt::Switch(_)
            | Stmt::Select(_)
            | Stmt::Raw(_) => {}
        }
    }
}
pub fn base_enum_name(input: &str) -> &str {
    input.split('<').next().unwrap_or(input).trim()
}
