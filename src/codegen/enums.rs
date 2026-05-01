use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn emit_enum(&mut self, enum_decl: &EnumDecl) -> String {
        if enum_decl.is_tagged() {
            self.emit_tagged_enum(enum_decl)
        } else {
            self.emit_simple_enum(enum_decl)
        }
    }

    pub(super) fn emit_simple_enum(&mut self, enum_decl: &EnumDecl) -> String {
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

    pub(super) fn emit_simple_enum_string_impl(&self, enum_decl: &EnumDecl) -> String {
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

    pub(super) fn emit_tagged_enum(&mut self, enum_decl: &EnumDecl) -> String {
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
            render_type_params_with_any(&enum_decl.type_params)
        ));
        out.push_str(&format!("\ttag {}\n", tag_name));
        for variant in &enum_decl.variants {
            for (idx, payload_ty) in variant.payload.iter().enumerate() {
                out.push_str(&format!(
                    "\t{}{} {}\n",
                    lower_ident(&variant.name),
                    idx,
                    render_type_ref(payload_ty)
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
                .map(|(idx, ty)| format!("v{} {}", idx, render_type_ref(ty)))
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
            self.import_binding("fmt", "fmt");
            out.push_str(&self.emit_tagged_enum_string_impl(enum_decl));
        }

        out.trim_end().to_string()
    }

    pub(super) fn emit_tagged_enum_string_impl(&self, enum_decl: &EnumDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "func (e {}{}) String() string {{\n",
            enum_decl.name,
            render_type_params(&enum_decl.type_params)
        ));
        let fmt_name = self
            .imports
            .get("fmt")
            .and_then(|alias| alias.as_ref())
            .filter(|alias| alias.as_str() != "_" && alias.as_str() != ".")
            .cloned()
            .unwrap_or_else(|| "fmt".to_string());
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
                let placeholders = std::iter::repeat_n("%v", variant.payload.len())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "\t\treturn {}.Sprintf(\"{}({})\"{})\n",
                    fmt_name, variant.name, placeholders, fmt_args
                ));
            }
        }
        out.push_str("\tdefault:\n");
        out.push_str(&format!("\t\treturn \"{}(?)\"\n\t}}\n}}", enum_decl.name));
        out
    }
}
