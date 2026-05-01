use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn emit_log_wrapper(
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
        out.push('}');
        out
    }

    pub(super) fn emit_retry_wrapper(
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

        out.push('}');
        out
    }

    pub(super) fn emit_memoize_wrapper(
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
            .map(render_type_ref)
            .unwrap_or_else(|| "interface{}".to_string());

        let mut out = String::new();
        out.push_str(&format!("type {} struct {{\n", key_type));
        for (idx, param) in sig.params.iter().enumerate() {
            out.push_str(&format!("\tP{} {}\n", idx, render_type_ref(&param.ty)));
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
        out.push('}');
        out
    }

    pub(super) fn emit_custom_wrapper(
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
        out.push('}');
        out
    }
}
