use super::*;

pub(super) fn validate_decorators(
    decorators: &[Decorator],
    ret_type: &ReturnType,
    params: &[ParamDecl],
    is_method: bool,
    known_functions: &HashMap<String, Vec<ParamDecl>>,
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
                validate_custom_decorator_signature(decorator, known_functions, diagnostics);
            }
        }
    }
}

fn validate_custom_decorator_signature(
    decorator: &Decorator,
    known_functions: &HashMap<String, Vec<ParamDecl>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if decorator.name.contains('.') {
        return;
    }
    let Some(params) = known_functions.get(&decorator.name) else {
        return;
    };
    let Some(next_param) = params.first() else {
        diagnostics.push(
            Diagnostic::new(
                format!(
                    "custom decorator `{}` must accept `next` as its first parameter",
                    decorator.name
                ),
                Some(decorator.span.clone()),
            )
            .with_code("E0300"),
        );
        return;
    };
    let next_ty = next_param.ty.raw.trim();
    if !(next_ty.starts_with("func(") || next_ty.starts_with("(fn(") || next_ty.starts_with("fn("))
    {
        diagnostics.push(
            Diagnostic::new(
                format!(
                    "custom decorator `{}` first parameter must be a function type",
                    decorator.name
                ),
                Some(next_param.span.clone()),
            )
            .with_code("E0301"),
        );
    }
    let expected_args = params.len().saturating_sub(1);
    if expected_args != decorator.args.len() {
        diagnostics.push(
            Diagnostic::new(
                format!(
                    "custom decorator `{}` expects {} argument(s), found {}",
                    decorator.name,
                    expected_args,
                    decorator.args.len()
                ),
                Some(decorator.span.clone()),
            )
            .with_code("E0302"),
        );
    }
}

pub(super) fn parse_positive_int(text: &str) -> Option<u64> {
    let cleaned = text.trim().replace('_', "");
    let value = cleaned.parse::<u64>().ok()?;
    if value == 0 { None } else { Some(value) }
}

pub(super) fn is_comparable_type(ty: &str) -> bool {
    let trimmed = ty.trim();
    !trimmed.starts_with("[]")
        && !trimmed.starts_with("map[")
        && !trimmed.starts_with("func(")
        && !trimmed.starts_with("fn(")
        && !trimmed.starts_with("(fn(")
        && !trimmed.contains("[]")
}
