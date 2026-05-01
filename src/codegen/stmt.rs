use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn emit_block(
        &mut self,
        block: &Block,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        let mut out = String::new();
        for stmt in &block.stmts {
            out.push_str(&self.emit_stmt(stmt, ret_type, indent));
        }
        out
    }
    pub(super) fn emit_stmt(
        &mut self,
        stmt: &Stmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        match stmt {
            Stmt::VarDecl(var_decl) => self.emit_var_decl(var_decl, ret_type, indent),
            Stmt::Return(ret_stmt) => self.emit_return(ret_stmt, ret_type, indent),
            Stmt::Expr(expr_stmt) => self.emit_expr_stmt(expr_stmt, ret_type, indent),
            Stmt::Match(match_stmt) => self.emit_match_stmt(match_stmt, ret_type, indent),
            Stmt::If(if_stmt) => self.emit_if_stmt(if_stmt, ret_type, indent),
            Stmt::Raw(raw) => format!("{}{}\n", tabs(indent), self.transform_expr(&raw.text)),
        }
    }

    pub(super) fn emit_var_decl(
        &mut self,
        var_decl: &VarDeclStmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        let mut out = String::new();
        let expr = self.transform_expr(&var_decl.expr.text);
        if var_decl.expr.has_try {
            let err = self.next_tmp("__gp_err");
            out.push_str(&format!(
                "{}{}, {} := {}\n",
                tabs(indent),
                var_decl.name,
                err,
                expr
            ));
            out.push_str(&format!("{}if {} != nil {{\n", tabs(indent), err));
            out.push_str(&self.emit_error_return_with_err_var(ret_type, &err, indent + 1, None));
            out.push_str(&format!("{}}}\n", tabs(indent)));
        } else {
            out.push_str(&format!("{}{} := {}\n", tabs(indent), var_decl.name, expr));
        }
        out
    }

    pub(super) fn emit_expr_stmt(
        &mut self,
        expr_stmt: &ExprStmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        let expr = self.transform_expr(&expr_stmt.expr.text);
        if expr_stmt.expr.has_try {
            self.needs_try_helper = true;
            let err = self.next_tmp("__gp_err");
            let mut out = String::new();
            out.push_str(&format!(
                "{}if {} := __goplusTry({}); {} != nil {{\n",
                tabs(indent),
                err,
                expr,
                err
            ));
            out.push_str(&self.emit_error_return_with_err_var(ret_type, &err, indent + 1, None));
            out.push_str(&format!("{}}}\n", tabs(indent)));
            out
        } else {
            format!("{}{}\n", tabs(indent), expr)
        }
    }

    pub(super) fn emit_return(
        &mut self,
        ret_stmt: &ReturnStmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        if ret_stmt.exprs.is_empty() {
            return match ret_type {
                ReturnType::ErrorOnly => format!("{}return nil\n", tabs(indent)),
                _ => format!("{}return\n", tabs(indent)),
            };
        }

        if ret_stmt.exprs.len() == 1 && ret_stmt.exprs[0].has_try {
            return self.emit_return_try(&ret_stmt.exprs[0], ret_type, indent);
        }

        let rendered = ret_stmt
            .exprs
            .iter()
            .map(|expr| self.transform_expr(&expr.text))
            .collect::<Vec<_>>();

        if ret_stmt.exprs.len() == 1 {
            let expr = rendered[0].clone();
            if is_error_ctor(&expr) {
                self.imports.insert("errors".to_string());
                let mapped = map_error_ctor(&expr);
                return match ret_type {
                    ReturnType::TypeWithError(ty) => {
                        format!(
                            "{}return {}, {}\n",
                            tabs(indent),
                            zero_value_expr(&ty.raw),
                            mapped
                        )
                    }
                    ReturnType::ErrorOnly => format!("{}return {}\n", tabs(indent), mapped),
                    _ => format!("{}return {}\n", tabs(indent), mapped),
                };
            }

            return match ret_type {
                ReturnType::TypeWithError(_) => {
                    format!("{}return {}, nil\n", tabs(indent), expr)
                }
                _ => format!("{}return {}\n", tabs(indent), expr),
            };
        }

        format!("{}return {}\n", tabs(indent), rendered.join(", "))
    }

    pub(super) fn emit_return_try(
        &mut self,
        expr: &Expr,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        let mut out = String::new();
        let transformed = self.transform_expr(&expr.text);
        match ret_type {
            ReturnType::TypeWithError(ty) => {
                let val = self.next_tmp("__gp_val");
                let err = self.next_tmp("__gp_err");
                out.push_str(&format!(
                    "{}{}, {} := {}\n",
                    tabs(indent),
                    val,
                    err,
                    transformed
                ));
                out.push_str(&format!("{}if {} != nil {{\n", tabs(indent), err));
                out.push_str(&self.emit_error_return_with_err_var(
                    ret_type,
                    &err,
                    indent + 1,
                    Some(ty),
                ));
                out.push_str(&format!("{}}}\n", tabs(indent)));
                out.push_str(&format!("{}return {}, nil\n", tabs(indent), val));
            }
            ReturnType::ErrorOnly => {
                self.needs_try_helper = true;
                let err = self.next_tmp("__gp_err");
                out.push_str(&format!(
                    "{}if {} := __goplusTry({}); {} != nil {{\n",
                    tabs(indent),
                    err,
                    transformed,
                    err
                ));
                out.push_str(&self.emit_error_return_with_err_var(
                    ret_type,
                    &err,
                    indent + 1,
                    None,
                ));
                out.push_str(&format!("{}}}\n", tabs(indent)));
                out.push_str(&format!("{}return nil\n", tabs(indent)));
            }
            _ => {
                out.push_str(&format!("{}return {}\n", tabs(indent), transformed));
            }
        }
        out
    }

    pub(super) fn emit_if_stmt(
        &mut self,
        if_stmt: &IfStmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{}if {} {{\n",
            tabs(indent),
            self.transform_expr(&if_stmt.condition.text)
        ));
        out.push_str(&self.emit_block(&if_stmt.then_block, ret_type, indent + 1));
        out.push_str(&format!("{}}}", tabs(indent)));
        if let Some(else_branch) = &if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => {
                    out.push_str(" else {\n");
                    out.push_str(&self.emit_block(block, ret_type, indent + 1));
                    out.push_str(&format!("{}}}\n", tabs(indent)));
                }
                ElseBranch::If(nested) => {
                    out.push_str(" else ");
                    out.push_str(self.emit_if_stmt(nested, ret_type, indent).trim_start());
                }
            }
        } else {
            out.push('\n');
        }
        out
    }

    pub(super) fn emit_match_stmt(
        &mut self,
        match_stmt: &MatchStmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        if let Some(enum_name) = &match_stmt.resolved_enum
            && let Some(enum_decl) = self.model.enums.get(enum_name)
        {
            if enum_decl.is_tagged() {
                return self.emit_tagged_match(match_stmt, ret_type, enum_decl, indent);
            }
            return self.emit_simple_match(match_stmt, ret_type, enum_decl, indent);
        }

        let mut out = String::new();
        out.push_str(&format!(
            "{}switch {} {{\n",
            tabs(indent),
            self.transform_expr(&match_stmt.value.text)
        ));
        for arm in &match_stmt.arms {
            match &arm.pattern {
                Pattern::Wildcard { .. } => {
                    out.push_str(&format!("{}default:\n", tabs(indent + 1)))
                }
                Pattern::TypedVariant {
                    enum_name, variant, ..
                } => out.push_str(&format!(
                    "{}case {}{}:\n",
                    tabs(indent + 1),
                    base_enum_name(enum_name),
                    variant
                )),
                Pattern::Variant { variant, .. } => {
                    out.push_str(&format!("{}case {}:\n", tabs(indent + 1), variant))
                }
            }
            out.push_str(&self.emit_match_arm_body(arm, ret_type, indent + 2));
        }
        out.push_str(&format!("{}}}\n", tabs(indent)));
        out
    }

    pub(super) fn emit_simple_match(
        &mut self,
        match_stmt: &MatchStmt,
        ret_type: &ReturnType,
        enum_decl: &EnumDecl,
        indent: usize,
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{}switch {} {{\n",
            tabs(indent),
            self.transform_expr(&match_stmt.value.text)
        ));

        let mut has_default = false;
        for arm in &match_stmt.arms {
            match &arm.pattern {
                Pattern::Wildcard { .. } => {
                    has_default = true;
                    out.push_str(&format!("{}default:\n", tabs(indent + 1)));
                }
                Pattern::TypedVariant { variant, .. } | Pattern::Variant { variant, .. } => {
                    out.push_str(&format!(
                        "{}case {}{}:\n",
                        tabs(indent + 1),
                        enum_decl.name,
                        variant
                    ));
                }
            }
            out.push_str(&self.emit_match_arm_body(arm, ret_type, indent + 2));
        }

        if !has_default {
            out.push_str(&format!(
                "{}default:\n{}panic(\"unreachable\")\n",
                tabs(indent + 1),
                tabs(indent + 2)
            ));
        }

        out.push_str(&format!("{}}}\n", tabs(indent)));
        out
    }

    pub(super) fn emit_tagged_match(
        &mut self,
        match_stmt: &MatchStmt,
        ret_type: &ReturnType,
        enum_decl: &EnumDecl,
        indent: usize,
    ) -> String {
        let mut out = String::new();
        let match_tmp = self.next_tmp("__gp_match");
        out.push_str(&format!(
            "{}{} := {}\n",
            tabs(indent),
            match_tmp,
            self.transform_expr(&match_stmt.value.text)
        ));
        out.push_str(&format!("{}switch {}.tag {{\n", tabs(indent), match_tmp));

        let mut has_default = false;
        for arm in &match_stmt.arms {
            match &arm.pattern {
                Pattern::Wildcard { .. } => {
                    has_default = true;
                    out.push_str(&format!("{}default:\n", tabs(indent + 1)));
                }
                Pattern::TypedVariant {
                    variant, bindings, ..
                }
                | Pattern::Variant {
                    variant, bindings, ..
                } => {
                    out.push_str(&format!(
                        "{}case {}Tag{}:\n",
                        tabs(indent + 1),
                        enum_decl.name,
                        variant
                    ));
                    if let Some(enum_variant) =
                        enum_decl.variants.iter().find(|v| v.name == *variant)
                    {
                        for (idx, binding) in bindings.iter().enumerate() {
                            if binding != "_" && idx < enum_variant.payload.len() {
                                out.push_str(&format!(
                                    "{}{} := {}.{}{}\n",
                                    tabs(indent + 2),
                                    binding,
                                    match_tmp,
                                    lower_ident(variant),
                                    idx
                                ));
                            }
                        }
                    }
                }
            }
            out.push_str(&self.emit_match_arm_body(arm, ret_type, indent + 2));
        }

        if !has_default {
            out.push_str(&format!(
                "{}default:\n{}panic(\"unreachable\")\n",
                tabs(indent + 1),
                tabs(indent + 2)
            ));
        }

        out.push_str(&format!("{}}}\n", tabs(indent)));
        out
    }

    pub(super) fn emit_match_arm_body(
        &mut self,
        arm: &MatchArm,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        match &arm.body {
            MatchArmBody::Expr(expr) => {
                let stmt = ReturnStmt {
                    exprs: vec![expr.clone()],
                    span: expr.span.clone(),
                };
                match ret_type {
                    ReturnType::Void => self.emit_expr_stmt(
                        &ExprStmt {
                            expr: expr.clone(),
                            span: expr.span.clone(),
                        },
                        ret_type,
                        indent,
                    ),
                    _ => self.emit_return(&stmt, ret_type, indent),
                }
            }
            MatchArmBody::Block(block) => self.emit_block(block, ret_type, indent),
        }
    }

    pub(super) fn emit_error_return_with_err_var(
        &self,
        ret_type: &ReturnType,
        err_var: &str,
        indent: usize,
        ty_hint: Option<&TypeRef>,
    ) -> String {
        match ret_type {
            ReturnType::ErrorOnly => format!("{}return {}\n", tabs(indent), err_var),
            ReturnType::TypeWithError(ty) => {
                let value_ty = ty_hint.unwrap_or(ty);
                format!(
                    "{}return {}, {}\n",
                    tabs(indent),
                    zero_value_expr(&value_ty.raw),
                    err_var
                )
            }
            _ => format!("{}return\n", tabs(indent)),
        }
    }

    pub(super) fn emit_try_helper(&self) -> String {
        "func __goplusTry(values ...any) error {\n\tif len(values) == 0 {\n\t\treturn nil\n\t}\n\tlast := values[len(values)-1]\n\tif last == nil {\n\t\treturn nil\n\t}\n\tif err, ok := last.(error); ok {\n\t\treturn err\n\t}\n\treturn fmt.Errorf(\"try expression must end with error\")\n}"
            .to_string()
    }
}
