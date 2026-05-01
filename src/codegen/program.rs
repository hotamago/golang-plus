use super::*;

impl<'a> GoGenerator<'a> {
    pub(super) fn emit_program(&mut self) -> String {
        for item in &self.program.items {
            let section = match item {
                Item::Struct(struct_decl) => self.emit_struct(struct_decl),
                Item::Enum(enum_decl) => self.emit_enum(enum_decl),
                Item::Function(function) => self.emit_function(function),
                Item::Impl(impl_block) => self.emit_impl_block(impl_block),
                Item::Raw(raw) => raw.text.clone(),
            };
            self.sections.push(section);
        }

        if self.needs_try_helper {
            self.import_binding("fmt", "fmt");
            self.sections.push(self.emit_try_helper());
        }

        if self.needs_main_wrapper {
            let fmt_name = self.import_binding("fmt", "fmt");
            let os_name = self.import_binding("os", "os");
            self.sections.push(format!(
                "func main() {{\n\tif err := mainWarp(); err != nil {{\n\t\t{fmt_name}.Fprintln({os_name}.Stderr, err)\n\t\t{os_name}.Exit(1)\n\t}}\n}}"
            ));
        }

        let mut out = String::new();
        out.push_str(&format!("package {}\n\n", self.program.package));
        if !self.imports.is_empty() {
            out.push_str("import (\n");
            for (path, alias) in &self.imports {
                match alias {
                    Some(alias) => out.push_str(&format!("\t{} \"{}\"\n", alias, path)),
                    None => out.push_str(&format!("\t\"{}\"\n", path)),
                }
            }
            out.push_str(")\n\n");
        }
        out.push_str(&self.sections.join("\n\n"));
        out.push('\n');
        out
    }
}
