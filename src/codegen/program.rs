use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn emit_program(&mut self) -> String {
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
}
