use super::*;

pub(super) fn validate_try_usage(
    expr: &Expr,
    ret_type: &ReturnType,
    diagnostics: &mut Vec<Diagnostic>,
) {
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
