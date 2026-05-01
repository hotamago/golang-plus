use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn emit_struct(&mut self, struct_decl: &StructDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!("type {} struct {{\n", struct_decl.name));
        for field in &struct_decl.fields {
            out.push_str(&format!(
                "\t{} {}\n",
                field.name,
                render_type_ref(&field.ty)
            ));
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

    pub(super) fn emit_struct_string_impl(&self, struct_decl: &StructDecl) -> String {
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

    pub(super) fn emit_impl_block(&mut self, impl_block: &ImplBlock) -> String {
        let mut out = String::new();
        for method in &impl_block.methods {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&self.emit_method(impl_block, method));
        }
        out
    }

    pub(super) fn emit_method(&mut self, impl_block: &ImplBlock, method: &MethodDecl) -> String {
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

    pub(super) fn emit_function(&mut self, function: &FnDecl) -> String {
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

    pub(super) fn emit_function_like(
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

    pub(super) fn emit_named_function(
        &mut self,
        sig: &FnSignature,
        go_name: &str,
        body: &Block,
    ) -> String {
        let mut out = String::new();
        out.push_str(&render_signature(sig, go_name));
        out.push_str(" {\n");
        out.push_str(&self.emit_block(body, &sig.ret, 1));
        out.push('}');
        out
    }

    pub(super) fn emit_forward_function(
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
        out.push('}');
        out
    }

    pub(super) fn emit_decorator_wrapper(
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
}
