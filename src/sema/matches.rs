use super::*;

pub(super) fn analyze_match_stmt(
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

    if resolved_enum.is_none()
        && let Some(found) = vars.get(match_stmt.value.text.trim())
    {
        resolved_enum = Some(found.clone());
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
                        enum_name,
                        variant,
                        bindings,
                        ..
                    } => {
                        if base_enum_name(enum_name) != enum_decl.name {
                            diagnostics.push(
                                Diagnostic::new(
                                    format!(
                                        "match arm uses enum `{}` but expected `{}`",
                                        base_enum_name(enum_name),
                                        enum_decl.name
                                    ),
                                    Some(arm.pattern.span()),
                                )
                                .with_code("E0202")
                                .with_hint(format!("use `{}::{variant}`", enum_decl.name)),
                            );
                        }
                        if !all_variants.contains(variant) {
                            diagnostics.push(
                                Diagnostic::new(
                                    format!(
                                        "unknown variant `{variant}` for enum `{}`",
                                        enum_decl.name
                                    ),
                                    Some(arm.pattern.span()),
                                )
                                .with_code("E0203")
                                .with_hint("check the enum variant name in the declaration"),
                            );
                        }
                        validate_match_variant_payload(
                            enum_decl,
                            variant,
                            bindings,
                            arm,
                            diagnostics,
                        );
                        if !seen.insert(variant.clone()) {
                            diagnostics.push(
                                Diagnostic::new(
                                    format!("duplicate match arm for variant `{variant}`"),
                                    Some(arm.span.clone()),
                                )
                                .with_code("E0200"),
                            );
                        }
                    }
                    Pattern::Variant {
                        variant, bindings, ..
                    } => {
                        if !all_variants.contains(variant) {
                            diagnostics.push(
                                Diagnostic::new(
                                    format!(
                                        "unknown variant `{variant}` for enum `{}`",
                                        enum_decl.name
                                    ),
                                    Some(arm.pattern.span()),
                                )
                                .with_code("E0203")
                                .with_hint("check the enum variant name in the declaration"),
                            );
                        }
                        validate_match_variant_payload(
                            enum_decl,
                            variant,
                            bindings,
                            arm,
                            diagnostics,
                        );
                        if !seen.insert(variant.clone()) {
                            diagnostics.push(
                                Diagnostic::new(
                                    format!("duplicate match arm for variant `{variant}`"),
                                    Some(arm.span.clone()),
                                )
                                .with_code("E0200"),
                            );
                        }
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
                    .with_code("E0204")
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
            .with_code("E0205")
            .with_hint("add explicit typed patterns, e.g. `Status::Pending`"),
        );
    }
}

fn validate_match_variant_payload(
    enum_decl: &EnumDecl,
    variant: &str,
    bindings: &[String],
    arm: &MatchArm,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(enum_variant) = enum_decl.variants.iter().find(|it| it.name == variant) else {
        return;
    };
    if enum_variant.payload.len() != bindings.len() {
        diagnostics.push(
            Diagnostic::new(
                format!(
                    "variant `{}` expects {} payload binding(s), found {}",
                    variant,
                    enum_variant.payload.len(),
                    bindings.len()
                ),
                Some(arm.pattern.span()),
            )
            .with_code("E0201")
            .with_hint("match payload bindings must match the variant payload count"),
        );
    }
}
pub(super) fn infer_enum_type_from_expr(
    expr: &str,
    enums: &HashMap<String, EnumDecl>,
) -> Option<String> {
    let (left, _) = expr.split_once("::")?;
    let candidate = base_enum_name(left.trim()).to_string();
    if enums.contains_key(&candidate) {
        Some(candidate)
    } else {
        None
    }
}
