use std::collections::BTreeSet;

use regex::Regex;

use crate::{
    ast::*,
    sema::{SemanticModel, base_enum_name},
};

pub fn generate_go(program: &Program, model: &SemanticModel) -> String {
    let mut generator = GoGenerator::new(program, model);
    generator.emit_program()
}

struct GoGenerator<'a> {
    program: &'a Program,
    model: &'a SemanticModel,
    imports: BTreeSet<String>,
    sections: Vec<String>,
    tmp_counter: usize,
    needs_try_helper: bool,
    needs_main_wrapper: bool,
}

impl<'a> GoGenerator<'a> {
    fn new(program: &'a Program, model: &'a SemanticModel) -> Self {
        Self {
            program,
            model,
            imports: program.imports.iter().cloned().collect(),
            sections: Vec::new(),
            tmp_counter: 0,
            needs_try_helper: false,
            needs_main_wrapper: false,
        }
    }

    fn emit_program(&mut self) -> String {
        for item in &self.program.items {
            let section = match item {
                Item::Struct(struct_decl) => self.emit_struct(struct_decl),
                Item::Enum(enum_decl) => self.emit_enum(enum_decl),
                Item::Function(function) => self.emit_function(function),
                Item::Impl(impl_block) => self.emit_impl_block(impl_block),
            };
            self.sections.push(section);
        }

        if self.needs_try_helper {
            self.imports.insert("fmt".to_string());
            self.sections.push(self.emit_try_helper());
        }

        if self.needs_main_wrapper {
            self.imports.insert("fmt".to_string());
            self.imports.insert("os".to_string());
            self.sections.push(
                "func main() {\n\tif err := mainWarp(); err != nil {\n\t\tfmt.Fprintln(os.Stderr, err)\n\t\tos.Exit(1)\n\t}\n}"
                    .to_string(),
            );
        }

        let mut out = String::new();
        out.push_str(&format!("package {}\n\n", self.program.package));
        if !self.imports.is_empty() {
            out.push_str("import (\n");
            for import in &self.imports {
                out.push_str(&format!("\t\"{}\"\n", import));
            }
            out.push_str(")\n\n");
        }
        out.push_str(&self.sections.join("\n\n"));
        out.push('\n');
        out
    }

    fn emit_struct(&mut self, struct_decl: &StructDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!("type {} struct {{\n", struct_decl.name));
        for field in &struct_decl.fields {
            out.push_str(&format!("\t{} {}\n", field.name, field.ty.raw.trim()));
        }
        out.push('}');

        if struct_decl
            .derives
            .iter()
            .any(|derive| matches!(derive, DeriveKind::String))
        {
            self.imports.insert("fmt".to_string());
            out.push_str("\n\n");
            out.push_str(&self.emit_struct_string_impl(struct_decl));
        }

        out
    }

    fn emit_struct_string_impl(&self, struct_decl: &StructDecl) -> String {
        let receiver = "s";
        if struct_decl.fields.is_empty() {
            return format!(
                "func ({receiver} {name}) String() string {{\n\treturn \"{name}{{}}\"\n}}",
                name = struct_decl.name
            );
        }

        let format_fields = struct_decl
            .fields
            .iter()
            .map(|field| format!("{}:%v", field.name))
            .collect::<Vec<_>>()
            .join(", ");
        let args = struct_decl
            .fields
            .iter()
            .map(|field| format!("{receiver}.{}", field.name))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "func ({receiver} {name}) String() string {{\n\treturn fmt.Sprintf(\"{name}{{{format_fields}}}\", {args})\n}}",
            name = struct_decl.name
        )
    }

    fn emit_enum(&mut self, enum_decl: &EnumDecl) -> String {
        if enum_decl.is_tagged() {
            self.emit_tagged_enum(enum_decl)
        } else {
            self.emit_simple_enum(enum_decl)
        }
    }

    fn emit_simple_enum(&mut self, enum_decl: &EnumDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!("type {} int\n\n", enum_decl.name));
        out.push_str("const (\n");
        for (idx, variant) in enum_decl.variants.iter().enumerate() {
            if idx == 0 {
                out.push_str(&format!(
                    "\t{}{} {} = iota\n",
                    enum_decl.name, variant.name, enum_decl.name
                ));
            } else {
                out.push_str(&format!("\t{}{}\n", enum_decl.name, variant.name));
            }
        }
        out.push(')');

        if enum_decl
            .derives
            .iter()
            .any(|derive| matches!(derive, DeriveKind::String))
        {
            out.push_str("\n\n");
            out.push_str(&self.emit_simple_enum_string_impl(enum_decl));
        }

        out
    }

    fn emit_simple_enum_string_impl(&self, enum_decl: &EnumDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!("func (e {}) String() string {{\n", enum_decl.name));
        out.push_str("\tswitch e {\n");
        for variant in &enum_decl.variants {
            out.push_str(&format!(
                "\tcase {}{}:\n\t\treturn \"{}\"\n",
                enum_decl.name, variant.name, variant.name
            ));
        }
        out.push_str("\tdefault:\n");
        out.push_str(&format!("\t\treturn \"{}(?)\"\n\t}}\n}}", enum_decl.name));
        out
    }

    fn emit_tagged_enum(&mut self, enum_decl: &EnumDecl) -> String {
        let mut out = String::new();
        let tag_name = format!("{}Tag", enum_decl.name);
        out.push_str(&format!("type {} int\n\n", tag_name));
        out.push_str("const (\n");
        for (idx, variant) in enum_decl.variants.iter().enumerate() {
            let const_name = format!("{}Tag{}", enum_decl.name, variant.name);
            if idx == 0 {
                out.push_str(&format!("\t{} {} = iota\n", const_name, tag_name));
            } else {
                out.push_str(&format!("\t{}\n", const_name));
            }
        }
        out.push_str(")\n\n");

        out.push_str(&format!(
            "type {}{} struct {{\n",
            enum_decl.name,
            render_type_params(&enum_decl.type_params)
        ));
        out.push_str(&format!("\ttag {}\n", tag_name));
        for variant in &enum_decl.variants {
            for (idx, payload_ty) in variant.payload.iter().enumerate() {
                out.push_str(&format!(
                    "\t{}{} {}\n",
                    lower_ident(&variant.name),
                    idx,
                    payload_ty.raw.trim()
                ));
            }
        }
        out.push_str("}\n\n");

        for variant in &enum_decl.variants {
            let ctor_name = format!("{}{}", enum_decl.name, variant.name);
            let type_params = render_type_params_with_any(&enum_decl.type_params);
            let instance_type_args = render_type_args(&enum_decl.type_params);
            let params = variant
                .payload
                .iter()
                .enumerate()
                .map(|(idx, ty)| format!("v{} {}", idx, ty.raw.trim()))
                .collect::<Vec<_>>()
                .join(", ");
            let mut inits = vec![format!("tag: {}Tag{}", enum_decl.name, variant.name)];
            for (idx, _) in variant.payload.iter().enumerate() {
                inits.push(format!("{}{}: v{}", lower_ident(&variant.name), idx, idx));
            }
            out.push_str(&format!(
                "func {ctor}{type_params}({params}) {name}{instance_type_args} {{\n\treturn {name}{instance_type_args}{{{inits}}}\n}}\n\n",
                ctor = ctor_name,
                type_params = type_params,
                params = params,
                name = enum_decl.name,
                instance_type_args = instance_type_args,
                inits = inits.join(", "),
            ));
        }

        if enum_decl
            .derives
            .iter()
            .any(|derive| matches!(derive, DeriveKind::String))
        {
            self.imports.insert("fmt".to_string());
            out.push_str(&self.emit_tagged_enum_string_impl(enum_decl));
        }

        out.trim_end().to_string()
    }

    fn emit_tagged_enum_string_impl(&self, enum_decl: &EnumDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "func (e {}{}) String() string {{\n",
            enum_decl.name,
            render_type_params_with_any(&enum_decl.type_params)
        ));
        out.push_str("\tswitch e.tag {\n");
        for variant in &enum_decl.variants {
            let tag = format!("{}Tag{}", enum_decl.name, variant.name);
            out.push_str(&format!("\tcase {}:\n", tag));
            if variant.payload.is_empty() {
                out.push_str(&format!("\t\treturn \"{}\"\n", variant.name));
            } else {
                let payload = variant
                    .payload
                    .iter()
                    .enumerate()
                    .map(|(idx, _)| format!("e.{}{}", lower_ident(&variant.name), idx))
                    .collect::<Vec<_>>()
                    .join(", ");
                let fmt_args = if payload.is_empty() {
                    String::new()
                } else {
                    format!(", {payload}")
                };
                let placeholders = std::iter::repeat("%v")
                    .take(variant.payload.len())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "\t\treturn fmt.Sprintf(\"{}({})\"{})\n",
                    variant.name, placeholders, fmt_args
                ));
            }
        }
        out.push_str("\tdefault:\n");
        out.push_str(&format!("\t\treturn \"{}(?)\"\n\t}}\n}}", enum_decl.name));
        out
    }
    fn emit_impl_block(&mut self, impl_block: &ImplBlock) -> String {
        let mut out = String::new();
        for method in &impl_block.methods {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&self.emit_method(impl_block, method));
        }
        out
    }

    fn emit_method(&mut self, impl_block: &ImplBlock, method: &MethodDecl) -> String {
        let receiver_type = match method.receiver {
            ReceiverKind::Value => impl_block.target.trim().to_string(),
            ReceiverKind::Pointer => format!("*{}", impl_block.target.trim()),
        };
        let method_sig = FnSignature {
            name: method.name.clone(),
            type_params: method.type_params.clone(),
            params: method.params.clone(),
            ret: method.ret.clone(),
            receiver: Some(("self".to_string(), receiver_type)),
        };

        self.emit_function_like(method_sig, &method.body, &method.decorators, false)
    }

    fn emit_function(&mut self, function: &FnDecl) -> String {
        let mut name = function.name.clone();
        if function.name == "main" && matches!(function.ret, ReturnType::ErrorOnly) {
            name = "mainWarp".to_string();
            self.needs_main_wrapper = true;
        }

        let signature = FnSignature {
            name,
            type_params: function.type_params.clone(),
            params: function.params.clone(),
            ret: function.ret.clone(),
            receiver: None,
        };

        self.emit_function_like(signature, &function.body, &function.decorators, true)
    }

    fn emit_function_like(
        &mut self,
        signature: FnSignature,
        body: &Block,
        decorators: &[Decorator],
        allow_memoize: bool,
    ) -> String {
        if decorators.is_empty() {
            return self.emit_named_function(&signature, &signature.name, body);
        }

        let mut out = String::new();
        let inner_name = format!("{}__inner", signature.name);
        out.push_str(&self.emit_named_function(&signature, &inner_name, body));

        let mut prev_name = inner_name;
        for (idx, decorator) in decorators.iter().rev().enumerate() {
            out.push_str("\n\n");
            let wrapper_name = format!("{}__decor{}", signature.name, idx);
            out.push_str(&self.emit_decorator_wrapper(
                &signature,
                &wrapper_name,
                &prev_name,
                decorator,
                allow_memoize,
            ));
            prev_name = wrapper_name;
        }

        out.push_str("\n\n");
        out.push_str(&self.emit_forward_function(&signature, &signature.name, &prev_name));
        out
    }

    fn emit_named_function(&mut self, sig: &FnSignature, go_name: &str, body: &Block) -> String {
        let mut out = String::new();
        out.push_str(&render_signature(sig, go_name));
        out.push_str(" {\n");
        out.push_str(&self.emit_block(body, &sig.ret, 1));
        out.push_str("}");
        out
    }

    fn emit_forward_function(
        &mut self,
        sig: &FnSignature,
        go_name: &str,
        target_name: &str,
    ) -> String {
        let mut out = String::new();
        out.push_str(&render_signature(sig, go_name));
        out.push_str(" {\n");
        let call = call_expr(sig, target_name);
        match sig.ret {
            ReturnType::Void => {
                out.push_str(&format!("\t{}\n", call));
            }
            _ => {
                out.push_str(&format!("\treturn {}\n", call));
            }
        }
        out.push_str("}");
        out
    }

    fn emit_decorator_wrapper(
        &mut self,
        sig: &FnSignature,
        wrapper_name: &str,
        prev_name: &str,
        decorator: &Decorator,
        allow_memoize: bool,
    ) -> String {
        match decorator.name.as_str() {
            "log" => self.emit_log_wrapper(sig, wrapper_name, prev_name),
            "retry" => self.emit_retry_wrapper(sig, wrapper_name, prev_name, decorator),
            "memoize" if allow_memoize => self.emit_memoize_wrapper(sig, wrapper_name, prev_name),
            _ => self.emit_custom_wrapper(sig, wrapper_name, prev_name, decorator),
        }
    }

    fn emit_log_wrapper(
        &mut self,
        sig: &FnSignature,
        wrapper_name: &str,
        prev_name: &str,
    ) -> String {
        self.imports.insert("fmt".to_string());
        let mut out = String::new();
        out.push_str(&render_signature(sig, wrapper_name));
        out.push_str(" {\n");
        out.push_str(&format!(
            "\tfmt.Printf(\"[goplus] enter {}\\n\")\n",
            sig.name
        ));
        let call = call_expr(sig, prev_name);
        match &sig.ret {
            ReturnType::Void => {
                out.push_str(&format!("\t{}\n", call));
                out.push_str(&format!(
                    "\tfmt.Printf(\"[goplus] exit {}\\n\")\n",
                    sig.name
                ));
            }
            ReturnType::Type(_) => {
                out.push_str(&format!("\tresult := {}\n", call));
                out.push_str(&format!(
                    "\tfmt.Printf(\"[goplus] exit {}\\n\")\n",
                    sig.name
                ));
                out.push_str("\treturn result\n");
            }
            ReturnType::ErrorOnly => {
                out.push_str(&format!("\terr := {}\n", call));
                out.push_str("\tif err != nil {\n");
                out.push_str(&format!(
                    "\t\tfmt.Printf(\"[goplus] error {}: %v\\n\", err)\n",
                    sig.name
                ));
                out.push_str("\t\treturn err\n\t}\n");
                out.push_str(&format!(
                    "\tfmt.Printf(\"[goplus] exit {}\\n\")\n",
                    sig.name
                ));
                out.push_str("\treturn nil\n");
            }
            ReturnType::TypeWithError(_) => {
                out.push_str(&format!("\tresult, err := {}\n", call));
                out.push_str("\tif err != nil {\n");
                out.push_str(&format!(
                    "\t\tfmt.Printf(\"[goplus] error {}: %v\\n\", err)\n",
                    sig.name
                ));
                out.push_str("\t\treturn result, err\n\t}\n");
                out.push_str(&format!(
                    "\tfmt.Printf(\"[goplus] exit {}\\n\")\n",
                    sig.name
                ));
                out.push_str("\treturn result, nil\n");
            }
        }
        out.push_str("}");
        out
    }

    fn emit_retry_wrapper(
        &mut self,
        sig: &FnSignature,
        wrapper_name: &str,
        prev_name: &str,
        decorator: &Decorator,
    ) -> String {
        self.imports.insert("time".to_string());
        let times = decorator
            .args
            .first()
            .and_then(|arg| arg.trim().replace('_', "").parse::<usize>().ok())
            .unwrap_or(1);
        let backoff = decorator
            .args
            .get(1)
            .and_then(|arg| arg.trim().replace('_', "").parse::<usize>().ok())
            .unwrap_or(0);

        let mut out = String::new();
        out.push_str(&render_signature(sig, wrapper_name));
        out.push_str(" {\n");
        let call = call_expr(sig, prev_name);

        match &sig.ret {
            ReturnType::ErrorOnly => {
                out.push_str("\tvar lastErr error\n");
                out.push_str(&format!(
                    "\tfor attempt := 0; attempt < {}; attempt++ {{\n",
                    times
                ));
                out.push_str(&format!("\t\tif err := {}; err == nil {{\n", call));
                out.push_str("\t\t\treturn nil\n\t\t} else {\n\t\t\tlastErr = err\n\t\t}\n");
                if backoff > 0 {
                    out.push_str(&format!(
                        "\t\ttime.Sleep(time.Duration({}) * time.Millisecond)\n",
                        backoff
                    ));
                }
                out.push_str("\t}\n\treturn lastErr\n");
            }
            ReturnType::TypeWithError(ty) => {
                out.push_str("\tvar lastErr error\n");
                out.push_str(&format!(
                    "\tfor attempt := 0; attempt < {}; attempt++ {{\n",
                    times
                ));
                out.push_str(&format!("\t\tresult, err := {}\n", call));
                out.push_str("\t\tif err == nil {\n\t\t\treturn result, nil\n\t\t}\n");
                out.push_str("\t\tlastErr = err\n");
                if backoff > 0 {
                    out.push_str(&format!(
                        "\t\ttime.Sleep(time.Duration({}) * time.Millisecond)\n",
                        backoff
                    ));
                }
                out.push_str("\t}\n");
                out.push_str(&self.emit_error_return_with_err_var(
                    &sig.ret,
                    "lastErr",
                    1,
                    Some(ty),
                ));
            }
            _ => {
                out.push_str(&format!("\treturn {}\n", call));
            }
        }

        out.push_str("}");
        out
    }

    fn emit_memoize_wrapper(
        &mut self,
        sig: &FnSignature,
        wrapper_name: &str,
        prev_name: &str,
    ) -> String {
        self.imports.insert("sync".to_string());
        let key_type = format!("{}Key", wrapper_name);
        let cache_name = format!("{}Cache", wrapper_name);
        let mu_name = format!("{}Mu", wrapper_name);
        let ret_ty = sig
            .ret
            .value_type()
            .map(|t| t.raw.trim().to_string())
            .unwrap_or_else(|| "interface{}".to_string());

        let mut out = String::new();
        out.push_str(&format!("type {} struct {{\n", key_type));
        for (idx, param) in sig.params.iter().enumerate() {
            out.push_str(&format!("\tP{} {}\n", idx, param.ty.raw.trim()));
        }
        out.push_str("}\n\n");
        out.push_str(&format!("var {} sync.Mutex\n", mu_name));
        out.push_str(&format!(
            "var {} = map[{}]{}{{}}\n\n",
            cache_name, key_type, ret_ty
        ));

        out.push_str(&render_signature(sig, wrapper_name));
        out.push_str(" {\n");
        let key_fields = sig
            .params
            .iter()
            .enumerate()
            .map(|(idx, param)| format!("P{}: {}", idx, param.name))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\tkey := {}{{{}}}\n", key_type, key_fields));
        out.push_str(&format!("\t{}.Lock()\n", mu_name));
        out.push_str(&format!("\tif cached, ok := {}[key]; ok {{\n", cache_name));
        out.push_str(&format!("\t\t{}.Unlock()\n", mu_name));
        out.push_str("\t\treturn cached\n\t}\n");
        out.push_str(&format!("\t{}.Unlock()\n", mu_name));

        let call = call_expr(sig, prev_name);
        out.push_str(&format!("\tresult := {}\n", call));
        out.push_str(&format!("\t{}.Lock()\n", mu_name));
        out.push_str(&format!("\t{}[key] = result\n", cache_name));
        out.push_str(&format!("\t{}.Unlock()\n", mu_name));
        out.push_str("\treturn result\n");
        out.push_str("}");
        out
    }

    fn emit_custom_wrapper(
        &mut self,
        sig: &FnSignature,
        wrapper_name: &str,
        prev_name: &str,
        decorator: &Decorator,
    ) -> String {
        let mut out = String::new();
        out.push_str(&render_signature(sig, wrapper_name));
        out.push_str(" {\n");

        let mut factory_args = vec![call_target_expr(sig, prev_name)];
        factory_args.extend(
            decorator
                .args
                .iter()
                .map(|arg| self.transform_expr(arg))
                .collect::<Vec<_>>(),
        );

        out.push_str(&format!(
            "\tdecorated := {}({})\n",
            decorator.name,
            factory_args.join(", ")
        ));
        let call = call_value_expr(sig, "decorated");
        match sig.ret {
            ReturnType::Void => out.push_str(&format!("\t{}\n", call)),
            _ => out.push_str(&format!("\treturn {}\n", call)),
        }
        out.push_str("}");
        out
    }

    fn emit_block(&mut self, block: &Block, ret_type: &ReturnType, indent: usize) -> String {
        let mut out = String::new();
        for stmt in &block.stmts {
            out.push_str(&self.emit_stmt(stmt, ret_type, indent));
        }
        out
    }
    fn emit_stmt(&mut self, stmt: &Stmt, ret_type: &ReturnType, indent: usize) -> String {
        match stmt {
            Stmt::VarDecl(var_decl) => self.emit_var_decl(var_decl, ret_type, indent),
            Stmt::Return(ret_stmt) => self.emit_return(ret_stmt, ret_type, indent),
            Stmt::Expr(expr_stmt) => self.emit_expr_stmt(expr_stmt, ret_type, indent),
            Stmt::Match(match_stmt) => self.emit_match_stmt(match_stmt, ret_type, indent),
            Stmt::If(if_stmt) => self.emit_if_stmt(if_stmt, ret_type, indent),
            Stmt::Raw(raw) => format!("{}{}\n", tabs(indent), self.transform_expr(&raw.text)),
        }
    }

    fn emit_var_decl(
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

    fn emit_expr_stmt(
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

    fn emit_return(
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

    fn emit_return_try(&mut self, expr: &Expr, ret_type: &ReturnType, indent: usize) -> String {
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

    fn emit_if_stmt(&mut self, if_stmt: &IfStmt, ret_type: &ReturnType, indent: usize) -> String {
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

    fn emit_match_stmt(
        &mut self,
        match_stmt: &MatchStmt,
        ret_type: &ReturnType,
        indent: usize,
    ) -> String {
        if let Some(enum_name) = &match_stmt.resolved_enum {
            if let Some(enum_decl) = self.model.enums.get(enum_name) {
                if enum_decl.is_tagged() {
                    return self.emit_tagged_match(match_stmt, ret_type, enum_decl, indent);
                }
                return self.emit_simple_match(match_stmt, ret_type, enum_decl, indent);
            }
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

    fn emit_simple_match(
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

    fn emit_tagged_match(
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

    fn emit_match_arm_body(
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

    fn emit_error_return_with_err_var(
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

    fn emit_try_helper(&self) -> String {
        "func __goplusTry(values ...any) error {\n\tif len(values) == 0 {\n\t\treturn nil\n\t}\n\tlast := values[len(values)-1]\n\tif last == nil {\n\t\treturn nil\n\t}\n\tif err, ok := last.(error); ok {\n\t\treturn err\n\t}\n\treturn fmt.Errorf(\"try expression must end with error\")\n}"
            .to_string()
    }

    fn transform_expr(&self, text: &str) -> String {
        let mut out = text.to_string();
        for enum_decl in self.model.enums.values() {
            for variant in &enum_decl.variants {
                if enum_decl.is_tagged() {
                    let re_generic = Regex::new(&format!(
                        r"\b{}\s*<([^>]*)>\s*::\s*{}\s*\(",
                        regex::escape(&enum_decl.name),
                        regex::escape(&variant.name)
                    ))
                    .expect("valid regex");
                    out = re_generic
                        .replace_all(&out, format!("{}{}<$1>(", enum_decl.name, variant.name))
                        .to_string();

                    let re_plain = Regex::new(&format!(
                        r"\b{}\s*::\s*{}\s*\(",
                        regex::escape(&enum_decl.name),
                        regex::escape(&variant.name)
                    ))
                    .expect("valid regex");
                    out = re_plain
                        .replace_all(&out, format!("{}{}(", enum_decl.name, variant.name))
                        .to_string();
                } else {
                    let re = Regex::new(&format!(
                        r"\b{}\s*::\s*{}\b",
                        regex::escape(&enum_decl.name),
                        regex::escape(&variant.name)
                    ))
                    .expect("valid regex");
                    out = re
                        .replace_all(&out, format!("{}{}", enum_decl.name, variant.name))
                        .to_string();
                }
            }
        }
        out
    }

    fn next_tmp(&mut self, prefix: &str) -> String {
        let id = self.tmp_counter;
        self.tmp_counter += 1;
        format!("{}{}", prefix, id)
    }
}

#[derive(Clone)]
struct FnSignature {
    name: String,
    type_params: Vec<String>,
    params: Vec<ParamDecl>,
    ret: ReturnType,
    receiver: Option<(String, String)>,
}

fn render_signature(sig: &FnSignature, go_name: &str) -> String {
    let receiver = sig
        .receiver
        .as_ref()
        .map(|(name, ty)| format!("({} {}) ", name, ty))
        .unwrap_or_default();
    let type_params = if sig.receiver.is_some() {
        String::new()
    } else {
        render_type_params_with_any(&sig.type_params)
    };
    let params = sig
        .params
        .iter()
        .map(|param| format!("{} {}", param.name, param.ty.raw.trim()))
        .collect::<Vec<_>>()
        .join(", ");
    let ret = render_return_type(&sig.ret);
    format!(
        "func {}{}{}({}){}",
        receiver, go_name, type_params, params, ret
    )
}

fn call_expr(sig: &FnSignature, target_name: &str) -> String {
    call_value_expr(sig, &call_target_expr(sig, target_name))
}

fn call_target_expr(sig: &FnSignature, target_name: &str) -> String {
    if sig.receiver.is_some() {
        format!("self.{}", target_name)
    } else {
        target_name.to_string()
    }
}

fn call_value_expr(sig: &FnSignature, callee_expr: &str) -> String {
    let args = sig
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({})", callee_expr, args)
}

fn render_return_type(ret: &ReturnType) -> String {
    match ret {
        ReturnType::Void => String::new(),
        ReturnType::Type(ty) => format!(" {}", ty.raw.trim()),
        ReturnType::ErrorOnly => " error".to_string(),
        ReturnType::TypeWithError(ty) => format!(" ({}, error)", ty.raw.trim()),
    }
}

fn render_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        return String::new();
    }
    format!("[{}]", type_params.join(", "))
}

fn render_type_params_with_any(type_params: &[String]) -> String {
    if type_params.is_empty() {
        return String::new();
    }
    let params = type_params
        .iter()
        .map(|param| format!("{} any", param))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", params)
}

fn render_type_args(type_params: &[String]) -> String {
    if type_params.is_empty() {
        return String::new();
    }
    format!("[{}]", type_params.join(", "))
}

fn lower_ident(input: &str) -> String {
    let mut chars = input.chars();
    if let Some(first) = chars.next() {
        format!(
            "{}{}",
            first.to_ascii_lowercase(),
            chars.collect::<String>()
        )
    } else {
        String::new()
    }
}

fn zero_value_expr(ty: &str) -> String {
    let trimmed = ty.trim();
    if trimmed == "string" {
        return "\"\"".to_string();
    }
    if trimmed == "bool" {
        return "false".to_string();
    }
    if matches!(
        trimmed,
        "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "float32"
            | "float64"
            | "byte"
            | "rune"
    ) {
        return "0".to_string();
    }
    if trimmed.starts_with('*')
        || trimmed.starts_with("[]")
        || trimmed.starts_with("map[")
        || trimmed.starts_with("func(")
        || trimmed == "error"
    {
        return "nil".to_string();
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && trimmed
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false)
    {
        return format!("{}{{}}", trimmed);
    }
    format!("{}{{}}", trimmed)
}

fn is_error_ctor(expr: &str) -> bool {
    expr.trim().starts_with("error(")
}

fn map_error_ctor(expr: &str) -> String {
    expr.replacen("error(", "errors.New(", 1)
}

fn tabs(indent: usize) -> String {
    "\t".repeat(indent)
}

#[cfg(test)]
mod tests {
    use crate::{parser::parse_program, sema::analyze};

    use super::generate_go;

    #[test]
    fn generates_retry_and_log_wrappers() {
        let src = r#"
package main

@log
@retry(3)
fn load(path: string) -> string! {
    value := read(path)?
    return value
}
"#;
        let mut program = parse_program(src).expect("parse ok");
        let model = analyze(&mut program).expect("sema ok");
        let go = generate_go(&program, &model);
        assert!(go.contains("func load__inner"));
        assert!(go.contains("func load__decor0"));
        assert!(go.contains("func load("));
        assert!(go.contains("for attempt := 0; attempt < 3; attempt++"));
        assert!(go.contains("[goplus] enter load"));
    }

    #[test]
    fn generates_custom_decorator_wrapper() {
        let src = r#"
package main

fn trace(next: func(path string) (string, error), label: string) -> func(path string) (string, error) {
    return next
}

@trace("io")
fn load(path: string) -> string! {
    return "ok"
}
"#;
        let mut program = parse_program(src).expect("parse ok");
        let model = analyze(&mut program).expect("sema ok");
        let go = generate_go(&program, &model);
        assert!(go.contains("decorated := trace(load__inner, \"io\")"));
        assert!(go.contains("return decorated(path)"));
    }

    #[test]
    fn maps_main_error_wrapper() {
        let src = r#"
package main

fn main() -> ! {
    return
}
"#;
        let mut program = parse_program(src).expect("parse ok");
        let model = analyze(&mut program).expect("sema ok");
        let go = generate_go(&program, &model);
        assert!(go.contains("func mainWarp() error"));
        assert!(go.contains("func main()"));
    }

    #[test]
    fn generates_memoize_cache() {
        let src = r#"
package main

@memoize
fn add(a: int, b: int) -> int {
    return a + b
}
"#;
        let mut program = parse_program(src).expect("parse ok");
        let model = analyze(&mut program).expect("sema ok");
        let go = generate_go(&program, &model);
        assert!(go.contains("type add__decor0Key struct"));
        assert!(go.contains("var add__decor0Cache"));
        assert!(go.contains("sync.Mutex"));
    }
}
