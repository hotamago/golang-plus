use super::*;

impl<'a> GoGenerator<'a> {
    /// Emit all derive implementations for a struct.
    pub(super) fn emit_struct_derives(&mut self, decl: &StructDecl) -> String {
        let mut out = String::new();
        for derive in &decl.derives {
            match derive {
                DeriveKind::String => {} // Handled in emit_struct directly
                DeriveKind::Debug => {
                    out.push_str("\n\n");
                    out.push_str(&self.emit_struct_debug(decl));
                }
                DeriveKind::Equal => {
                    out.push_str("\n\n");
                    out.push_str(&self.emit_struct_equal(decl));
                }
                DeriveKind::JsonMarshal => {
                    out.push_str("\n\n");
                    out.push_str(&self.emit_struct_json_marshal(decl));
                }
                DeriveKind::JsonUnmarshal => {
                    out.push_str("\n\n");
                    out.push_str(&self.emit_struct_json_unmarshal(decl));
                }
            }
        }
        out
    }

    /// Emit all derive implementations for an enum.
    pub(super) fn emit_enum_derives(&mut self, decl: &EnumDecl) -> String {
        let mut out = String::new();
        for derive in &decl.derives {
            match derive {
                DeriveKind::String => {} // Handled in emit_enum directly
                DeriveKind::Debug => {
                    out.push_str("\n\n");
                    if decl.is_tagged() {
                        out.push_str(&self.emit_tagged_enum_debug(decl));
                    } else {
                        out.push_str(&self.emit_simple_enum_debug(decl));
                    }
                }
                DeriveKind::Equal => {
                    out.push_str("\n\n");
                    if decl.is_tagged() {
                        out.push_str(&self.emit_tagged_enum_equal(decl));
                    } else {
                        out.push_str(&self.emit_simple_enum_equal(decl));
                    }
                }
                DeriveKind::JsonMarshal => {
                    out.push_str("\n\n");
                    out.push_str(&self.emit_enum_json_marshal(decl));
                }
                DeriveKind::JsonUnmarshal => {
                    out.push_str("\n\n");
                    out.push_str(&self.emit_enum_json_unmarshal(decl));
                }
            }
        }
        out
    }

    // --- Debug derive ---

    fn emit_struct_debug(&mut self, decl: &StructDecl) -> String {
        let fmt_name = self.import_binding("fmt", "fmt");
        if decl.fields.is_empty() {
            return format!(
                "func (s {name}) Debug() string {{\n\treturn {fmt}.Sprintf(\"{name}{{}}\")\n}}",
                name = decl.name,
                fmt = fmt_name
            );
        }
        let format_parts: Vec<String> = decl
            .fields
            .iter()
            .map(|f| format!("{}:%+v", f.name))
            .collect();
        let args: Vec<String> = decl
            .fields
            .iter()
            .map(|f| format!("s.{}", f.name))
            .collect();
        format!(
            "func (s {name}) Debug() string {{\n\treturn {fmt}.Sprintf(\"{name}{{{parts}}}\", {args})\n}}",
            name = decl.name,
            fmt = fmt_name,
            parts = format_parts.join(", "),
            args = args.join(", ")
        )
    }

    fn emit_simple_enum_debug(&mut self, decl: &EnumDecl) -> String {
        let fmt_name = self.import_binding("fmt", "fmt");
        let mut out = String::new();
        out.push_str(&format!(
            "func (e {}) Debug() string {{\n\treturn {}.Sprintf(\"%s(%d)\", e.String(), int(e))\n}}",
            decl.name, fmt_name
        ));
        out
    }

    fn emit_tagged_enum_debug(&mut self, decl: &EnumDecl) -> String {
        let fmt_name = self.import_binding("fmt", "fmt");
        let mut out = String::new();
        out.push_str(&format!(
            "func (e {}{}) Debug() string {{\n",
            decl.name,
            render_type_params(&decl.type_params)
        ));
        out.push_str("\tswitch e.tag {\n");
        for variant in &decl.variants {
            let tag = format!("{}Tag{}", decl.name, variant.name);
            out.push_str(&format!("\tcase {}:\n", tag));
            if variant.payload.is_empty() {
                out.push_str(&format!(
                    "\t\treturn \"{}::{}()\"\n",
                    decl.name, variant.name
                ));
            } else {
                let fields: Vec<String> = variant
                    .payload
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("e.{}{}", lower_ident(&variant.name), i))
                    .collect();
                let placeholders: Vec<&str> = variant.payload.iter().map(|_| "%+v").collect();
                out.push_str(&format!(
                    "\t\treturn {}.Sprintf(\"{}::{}({})\", {})\n",
                    fmt_name,
                    decl.name,
                    variant.name,
                    placeholders.join(", "),
                    fields.join(", ")
                ));
            }
        }
        out.push_str(&format!(
            "\tdefault:\n\t\treturn \"{}(?)\"\n\t}}\n}}",
            decl.name
        ));
        out
    }

    // --- Equal derive ---

    fn emit_struct_equal(&self, decl: &StructDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "func (s {name}) Equal(other {name}) bool {{\n",
            name = decl.name
        ));
        if decl.fields.is_empty() {
            out.push_str("\treturn true\n}");
            return out;
        }
        let conditions: Vec<String> = decl
            .fields
            .iter()
            .map(|f| format!("s.{} == other.{}", f.name, f.name))
            .collect();
        out.push_str(&format!("\treturn {}\n}}", conditions.join(" && ")));
        out
    }

    fn emit_simple_enum_equal(&self, decl: &EnumDecl) -> String {
        format!(
            "func (e {name}) Equal(other {name}) bool {{\n\treturn e == other\n}}",
            name = decl.name
        )
    }

    fn emit_tagged_enum_equal(&self, decl: &EnumDecl) -> String {
        let type_params = render_type_params(&decl.type_params);
        let mut out = String::new();
        out.push_str(&format!(
            "func (e {name}{tp}) Equal(other {name}{tp}) bool {{\n",
            name = decl.name,
            tp = type_params
        ));
        out.push_str("\tif e.tag != other.tag {\n\t\treturn false\n\t}\n");
        out.push_str("\tswitch e.tag {\n");
        for variant in &decl.variants {
            let tag = format!("{}Tag{}", decl.name, variant.name);
            out.push_str(&format!("\tcase {}:\n", tag));
            if variant.payload.is_empty() {
                out.push_str("\t\treturn true\n");
            } else {
                let conditions: Vec<String> = variant
                    .payload
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let field = format!("{}{}", lower_ident(&variant.name), i);
                        format!("e.{} == other.{}", field, field)
                    })
                    .collect();
                out.push_str(&format!("\t\treturn {}\n", conditions.join(" && ")));
            }
        }
        out.push_str("\tdefault:\n\t\treturn false\n\t}\n}");
        out
    }

    // --- JSON derives ---

    fn emit_struct_json_marshal(&mut self, decl: &StructDecl) -> String {
        let json_name = self.import_binding("encoding/json", "json");
        format!(
            "func (s {name}) MarshalJSON() ([]byte, error) {{\n\treturn {json}.Marshal(struct {{\n{fields}\n\t}}{{{inits}}})\n}}",
            name = decl.name,
            json = json_name,
            fields = decl
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "\t\t{} {} `json:\"{}\"`",
                        f.name,
                        render_type_ref(&f.ty),
                        to_json_key(&f.name)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            inits = decl
                .fields
                .iter()
                .map(|f| format!("s.{}", f.name))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn emit_struct_json_unmarshal(&mut self, decl: &StructDecl) -> String {
        let json_name = self.import_binding("encoding/json", "json");
        let mut out = String::new();
        out.push_str(&format!(
            "func (s *{name}) UnmarshalJSON(data []byte) error {{\n",
            name = decl.name
        ));
        out.push_str("\tvar raw struct {\n");
        for field in &decl.fields {
            out.push_str(&format!(
                "\t\t{} {} `json:\"{}\"`\n",
                field.name,
                render_type_ref(&field.ty),
                to_json_key(&field.name)
            ));
        }
        out.push_str("\t}\n");
        out.push_str(&format!(
            "\tif err := {}.Unmarshal(data, &raw); err != nil {{\n\t\treturn err\n\t}}\n",
            json_name
        ));
        for field in &decl.fields {
            out.push_str(&format!("\ts.{} = raw.{}\n", field.name, field.name));
        }
        out.push_str("\treturn nil\n}");
        out
    }

    fn emit_enum_json_marshal(&mut self, decl: &EnumDecl) -> String {
        let json_name = self.import_binding("encoding/json", "json");
        if decl.is_tagged() {
            let mut out = String::new();
            out.push_str(&format!(
                "func (e {}{}) MarshalJSON() ([]byte, error) {{\n",
                decl.name,
                render_type_params(&decl.type_params)
            ));
            out.push_str("\tswitch e.tag {\n");
            for variant in &decl.variants {
                let tag = format!("{}Tag{}", decl.name, variant.name);
                out.push_str(&format!("\tcase {}:\n", tag));
                if variant.payload.is_empty() {
                    out.push_str(&format!(
                        "\t\treturn {}.Marshal(map[string]any{{\"type\": \"{}\"}})\n",
                        json_name, variant.name
                    ));
                } else {
                    let fields: Vec<String> = variant
                        .payload
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            format!("\"value{}\": e.{}{}", i, lower_ident(&variant.name), i)
                        })
                        .collect();
                    out.push_str(&format!(
                        "\t\treturn {}.Marshal(map[string]any{{\"type\": \"{}\", {}}})\n",
                        json_name,
                        variant.name,
                        fields.join(", ")
                    ));
                }
            }
            out.push_str(&format!(
                "\tdefault:\n\t\treturn nil, {}.Errorf(\"unknown {} tag: %d\", e.tag)\n\t}}\n}}",
                json_name, decl.name
            ));
            out
        } else {
            format!(
                "func (e {name}) MarshalJSON() ([]byte, error) {{\n\treturn {json}.Marshal(e.String())\n}}",
                name = decl.name,
                json = json_name
            )
        }
    }

    fn emit_enum_json_unmarshal(&mut self, decl: &EnumDecl) -> String {
        let json_name = self.import_binding("encoding/json", "json");
        if decl.is_tagged() {
            let mut out = String::new();
            out.push_str(&format!(
                "func (e *{}{}) UnmarshalJSON(data []byte) error {{\n",
                decl.name,
                render_type_params(&decl.type_params)
            ));
            out.push_str(&format!(
                "\tvar raw map[string]{}interface{{}}{}\n",
                "{", "}"
            ));
            out.push_str(&format!(
                "\tif err := {}.Unmarshal(data, &raw); err != nil {{\n\t\treturn err\n\t}}\n",
                json_name
            ));
            out.push_str("\tswitch raw[\"type\"] {\n");
            for variant in &decl.variants {
                out.push_str(&format!("\tcase \"{}\":\n", variant.name));
                out.push_str(&format!("\t\te.tag = {}Tag{}\n", decl.name, variant.name));
            }
            out.push_str(&format!(
                "\tdefault:\n\t\treturn {}.Errorf(\"unknown {} type: %v\", raw[\"type\"])\n\t}}\n",
                json_name, decl.name
            ));
            out.push_str("\treturn nil\n}");
            out
        } else {
            let mut out = String::new();
            out.push_str(&format!(
                "func (e *{name}) UnmarshalJSON(data []byte) error {{\n",
                name = decl.name
            ));
            out.push_str("\tvar s string\n");
            out.push_str(&format!(
                "\tif err := {}.Unmarshal(data, &s); err != nil {{\n\t\treturn err\n\t}}\n",
                json_name
            ));
            out.push_str("\tswitch s {\n");
            for variant in &decl.variants {
                out.push_str(&format!(
                    "\tcase \"{}\":\n\t\t*e = {}{}\n",
                    variant.name, decl.name, variant.name
                ));
            }
            out.push_str(&format!(
                "\tdefault:\n\t\treturn {}.Errorf(\"unknown {} value: %s\", s)\n\t}}\n",
                json_name, decl.name
            ));
            out.push_str("\treturn nil\n}");
            out
        }
    }
}

fn to_json_key(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i == 0 {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
