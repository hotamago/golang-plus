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
                    diagnostics.push(
                        Diagnostic::new(
                            "`@retry` requires function returning `!` or `T!`",
                            Some(decorator.span.clone()),
                        )
                        .with_code("E0303")
                        .with_hint("change the return type to `!` or `T!`, or remove `@retry`"),
                    );
                }
                if decorator.args.is_empty() || decorator.args.len() > 2 {
                    diagnostics.push(
                        Diagnostic::new(
                            "`@retry` expects `@retry(times)` or `@retry(times, backoff_ms)`",
                            Some(decorator.span.clone()),
                        )
                        .with_code("E0304")
                        .with_hint("example: @retry(3, 100)"),
                    );
                }
                for arg in &decorator.args {
                    if parse_positive_int(arg).is_none() {
                        diagnostics.push(
                            Diagnostic::new(
                                format!("`@retry` argument `{arg}` must be a positive integer"),
                                Some(decorator.span.clone()),
                            )
                            .with_code("E0305")
                            .with_hint("use a positive integer literal such as `3`"),
                        );
                    }
                }
            }
            "memoize" => {
                if is_method {
                    diagnostics.push(
                        Diagnostic::new(
                            "`@memoize` is allowed only on top-level functions",
                            Some(decorator.span.clone()),
                        )
                        .with_code("E0306")
                        .with_hint("move the function out of `impl` or remove `@memoize`"),
                    );
                }
                if !matches!(ret_type, ReturnType::Type(_)) {
                    diagnostics.push(
                        Diagnostic::new(
                            "`@memoize` requires non-error return type `T`",
                            Some(decorator.span.clone()),
                        )
                        .with_code("E0307")
                        .with_hint("use a single non-error return value"),
                    );
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
                            .with_code("E0308")
                            .with_hint("use scalar/pointer/named comparable types only"),
                        );
                    }
                }
            }
            _ => {
                validate_custom_decorator_signature(
                    decorator,
                    params,
                    ret_type,
                    known_functions,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_custom_decorator_signature(
    decorator: &Decorator,
    decorated_params: &[ParamDecl],
    decorated_ret: &ReturnType,
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
        return;
    }

    // Check argument count
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

    // Strong type shape validation: parse the `next` function type and compare
    // against the decorated function's actual signature.
    validate_next_type_shape(
        decorator,
        next_ty,
        decorated_params,
        decorated_ret,
        diagnostics,
    );
}

/// Validate that the `next` parameter's function type matches the decorated function's signature.
fn validate_next_type_shape(
    decorator: &Decorator,
    next_ty: &str,
    decorated_params: &[ParamDecl],
    decorated_ret: &ReturnType,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Parse the next type: func(params...) return_type
    let Some((param_part, ret_part)) = parse_func_type_parts(next_ty) else {
        return; // Can't parse, skip deep validation
    };

    // Count parameters in the next type
    let next_param_count = count_func_params(&param_part);
    let decorated_param_count = decorated_params.len();

    if next_param_count != decorated_param_count {
        diagnostics.push(
            Diagnostic::new(
                format!(
                    "decorator `{}` `next` function expects {} parameter(s), but decorated function has {}",
                    decorator.name, next_param_count, decorated_param_count
                ),
                Some(decorator.span.clone()),
            )
            .with_code("E0310")
            .with_hint("the `next` parameter's function type must match the decorated function signature"),
        );
    }

    // Check return type shape
    let decorated_is_error = decorated_ret.is_error_capable();
    let ret_clean = ret_part.strip_prefix("->").unwrap_or(&ret_part).trim();
    let next_has_error = ret_clean.contains("error") || ret_clean.contains('!');
    let next_has_value =
        !ret_clean.is_empty() && ret_clean != "error" && ret_clean != "!" && ret_clean != "(error)";

    match decorated_ret {
        ReturnType::Void => {
            if !ret_part.is_empty() {
                diagnostics.push(
                    Diagnostic::new(
                        format!(
                            "decorator `{}` `next` function has return type but decorated function is void",
                            decorator.name
                        ),
                        Some(decorator.span.clone()),
                    )
                    .with_code("E0309")
                    .with_hint("the `next` function type should have no return for void functions"),
                );
            }
        }
        ReturnType::ErrorOnly => {
            if !next_has_error {
                diagnostics.push(
                    Diagnostic::new(
                        format!(
                            "decorator `{}` `next` function should return `error` to match decorated function",
                            decorator.name
                        ),
                        Some(decorator.span.clone()),
                    )
                    .with_code("E0309"),
                );
            }
        }
        ReturnType::TypeWithError(_) | ReturnType::GoTypeWithError(_) => {
            if !next_has_error || !next_has_value {
                diagnostics.push(
                    Diagnostic::new(
                        format!(
                            "decorator `{}` `next` function should return `(T, error)` to match decorated function",
                            decorator.name
                        ),
                        Some(decorator.span.clone()),
                    )
                    .with_code("E0309"),
                );
            }
        }
        ReturnType::Type(_) => {
            if decorated_is_error != next_has_error {
                diagnostics.push(
                    Diagnostic::new(
                        format!(
                            "decorator `{}` `next` function return type does not match decorated function",
                            decorator.name
                        ),
                        Some(decorator.span.clone()),
                    )
                    .with_code("E0309"),
                );
            }
        }
    }
}

/// Parse `func(...)` or `func(...) (...)` into (param_str, return_str).
fn parse_func_type_parts(ty: &str) -> Option<(String, String)> {
    let ty = ty.trim();
    let stripped = ty
        .strip_prefix("func(")
        .or_else(|| ty.strip_prefix("fn("))
        .or_else(|| ty.strip_prefix("(fn("))?;

    // Find matching paren
    let mut depth = 1usize;
    let mut end = 0;
    for (i, ch) in stripped.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }

    let param_part = stripped[..end].to_string();
    let rest = stripped[end + 1..].trim().to_string();
    // Handle closing paren if it was `(fn(...) ...)`
    let rest = rest.strip_suffix(')').unwrap_or(&rest).trim().to_string();

    Some((param_part, rest))
}

/// Count the number of parameters in a function param string.
fn count_func_params(params: &str) -> usize {
    let trimmed = params.trim();
    if trimmed.is_empty() {
        return 0;
    }
    // Count top-level commas
    let mut count = 1usize;
    let mut depth = 0usize;
    for ch in trimmed.chars() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    count
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_func_type_basic() {
        let (params, ret) = parse_func_type_parts("func(path string) (string, error)").unwrap();
        assert_eq!(params, "path string");
        assert!(ret.contains("string"));
        assert!(ret.contains("error"));
    }

    #[test]
    fn count_params_empty() {
        assert_eq!(count_func_params(""), 0);
    }

    #[test]
    fn count_params_two() {
        assert_eq!(count_func_params("a string, b int"), 2);
    }

    #[test]
    fn count_params_nested() {
        assert_eq!(count_func_params("a func(int, int), b string"), 2);
    }
}
