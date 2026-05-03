use crate::ast::*;

pub fn format_gp(program: &Program, _original_source: &str) -> String {
    let mut f = GpFormatter::new();
    f.emit_program(program);
    f.output
}

struct GpFormatter {
    output: String,
}

impl GpFormatter {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn emit_program(&mut self, program: &Program) {
        self.output
            .push_str(&format!("package {}\n", program.package));

        if !program.imports.is_empty() {
            self.output.push('\n');
            if program.imports.len() == 1 {
                self.emit_import_single(&program.imports[0]);
            } else {
                self.emit_import_group(&program.imports);
            }
        }

        for item in &program.items {
            self.output.push('\n');
            self.emit_item(item);
            self.output.push('\n');
        }
    }

    fn emit_import_single(&mut self, import: &ImportDecl) {
        self.output.push_str("import ");
        if let Some(alias) = &import.alias {
            self.output.push_str(alias);
            self.output.push(' ');
        }
        self.output.push_str(&format!("\"{}\"", import.path));
        self.output.push('\n');
    }

    fn emit_import_group(&mut self, imports: &[ImportDecl]) {
        self.output.push_str("import (\n");
        for import in imports {
            self.output.push('\t');
            if let Some(alias) = &import.alias {
                self.output.push_str(alias);
                self.output.push(' ');
            }
            self.output.push_str(&format!("\"{}\"", import.path));
            self.output.push('\n');
        }
        self.output.push_str(")\n");
    }

    fn emit_item(&mut self, item: &Item) {
        match item {
            Item::Struct(decl) => self.emit_struct(decl),
            Item::Enum(decl) => self.emit_enum(decl),
            Item::Function(decl) => self.emit_function(decl),
            Item::Impl(decl) => self.emit_impl(decl),
            Item::Raw(decl) => self.emit_raw(decl),
        }
    }

    fn emit_derives(&mut self, derives: &[DeriveKind]) {
        if derives.is_empty() {
            return;
        }
        let names: Vec<&str> = derives
            .iter()
            .map(|d| match d {
                DeriveKind::String => "String",
                DeriveKind::Debug => "Debug",
                DeriveKind::Equal => "Equal",
                DeriveKind::JsonMarshal => "JsonMarshal",
                DeriveKind::JsonUnmarshal => "JsonUnmarshal",
            })
            .collect();
        self.output
            .push_str(&format!("@derive({})\n", names.join(", ")));
    }

    fn emit_struct(&mut self, decl: &StructDecl) {
        self.emit_derives(&decl.derives);
        self.output.push_str(&format!("struct {} {{\n", decl.name));
        for field in &decl.fields {
            let tag = field
                .tag
                .as_ref()
                .map(|tag| format!(" {}", tag))
                .unwrap_or_default();
            self.output
                .push_str(&format!("\t{}: {}{}\n", field.name, field.ty.raw, tag));
        }
        self.output.push('}');
    }

    fn emit_enum(&mut self, decl: &EnumDecl) {
        self.emit_derives(&decl.derives);
        self.output.push_str("enum ");
        self.output.push_str(&decl.name);
        if !decl.type_params.is_empty() {
            self.output.push('<');
            self.output.push_str(&decl.type_params.join(", "));
            self.output.push('>');
        }
        self.output.push_str(" {\n");
        for variant in &decl.variants {
            self.output.push('\t');
            self.output.push_str(&variant.name);
            if !variant.payload.is_empty() {
                self.output.push('(');
                let types: Vec<&str> = variant.payload.iter().map(|ty| ty.raw.as_str()).collect();
                self.output.push_str(&types.join(", "));
                self.output.push(')');
            }
            self.output.push('\n');
        }
        self.output.push('}');
    }

    fn emit_decorators(&mut self, decorators: &[Decorator]) {
        for dec in decorators {
            self.output.push('@');
            self.output.push_str(&dec.name);
            if !dec.args.is_empty() {
                self.output.push('(');
                self.output.push_str(&dec.args.join(", "));
                self.output.push(')');
            }
            self.output.push('\n');
        }
    }

    fn emit_function(&mut self, decl: &FnDecl) {
        self.emit_decorators(&decl.decorators);
        self.output.push_str("fn ");
        self.output.push_str(&decl.name);
        if !decl.type_params.is_empty() {
            self.output.push('<');
            self.output.push_str(&decl.type_params.join(", "));
            self.output.push('>');
        }
        self.output.push('(');
        let params: Vec<String> = decl
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.ty.raw))
            .collect();
        self.output.push_str(&params.join(", "));
        self.output.push(')');
        self.emit_return_type(&decl.ret);
        self.output.push_str(" {\n");
        self.emit_block(&decl.body, 1);
        self.output.push('}');
    }

    fn emit_impl(&mut self, decl: &ImplBlock) {
        self.output.push_str(&format!("impl {} {{\n", decl.target));
        for (idx, method) in decl.methods.iter().enumerate() {
            if idx > 0 {
                self.output.push('\n');
            }
            self.emit_method(method);
        }
        self.output.push('}');
    }

    fn emit_method(&mut self, method: &MethodDecl) {
        self.output.push('\t');
        for dec in &method.decorators {
            self.output.push('@');
            self.output.push_str(&dec.name);
            if !dec.args.is_empty() {
                self.output.push('(');
                self.output.push_str(&dec.args.join(", "));
                self.output.push(')');
            }
            self.output.push('\n');
            self.output.push('\t');
        }
        match method.receiver {
            ReceiverKind::Pointer => self.output.push_str("fn mut "),
            ReceiverKind::Value => self.output.push_str("fn "),
        }
        self.output.push_str(&method.name);
        self.output.push('(');
        let params: Vec<String> = method
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.ty.raw))
            .collect();
        self.output.push_str(&params.join(", "));
        self.output.push(')');
        self.emit_return_type(&method.ret);
        self.output.push_str(" {\n");
        self.emit_block(&method.body, 2);
        self.output.push('\t');
        self.output.push_str("}\n");
    }

    fn emit_return_type(&mut self, ret: &ReturnType) {
        match ret {
            ReturnType::Void => {}
            ReturnType::Type(ty) => {
                self.output.push_str(" -> ");
                self.output.push_str(&ty.raw);
            }
            ReturnType::ErrorOnly => {
                self.output.push_str(" -> !");
            }
            ReturnType::TypeWithError(ty) => {
                self.output.push_str(" -> ");
                self.output.push_str(&ty.raw);
                self.output.push('!');
            }
        }
    }

    fn emit_block(&mut self, block: &Block, indent: usize) {
        let tabs = "\t".repeat(indent);
        for stmt in &block.stmts {
            self.emit_stmt(stmt, &tabs, indent);
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt, tabs: &str, indent: usize) {
        match stmt {
            Stmt::VarDecl(v) => {
                let expr_text = if v.expr.has_try {
                    format!("{}?", v.expr.text)
                } else {
                    v.expr.text.clone()
                };
                self.output
                    .push_str(&format!("{}{} := {}\n", tabs, v.name, expr_text));
            }
            Stmt::Assign(a) => {
                self.output.push_str(&format!("{}{}\n", tabs, a.text));
            }
            Stmt::Return(r) => {
                if r.exprs.is_empty() {
                    self.output.push_str(&format!("{}return\n", tabs));
                } else {
                    let exprs: Vec<String> = r
                        .exprs
                        .iter()
                        .map(|e| {
                            if e.has_try {
                                format!("{}?", e.text)
                            } else {
                                e.text.clone()
                            }
                        })
                        .collect();
                    self.output
                        .push_str(&format!("{}return {}\n", tabs, exprs.join(", ")));
                }
            }
            Stmt::Defer(raw) => self.output.push_str(&format!("{}{}\n", tabs, raw.text)),
            Stmt::Go(raw) => self.output.push_str(&format!("{}{}\n", tabs, raw.text)),
            Stmt::For(raw) => self.output.push_str(&format!("{}{}\n", tabs, raw.text)),
            Stmt::Switch(raw) => self.output.push_str(&format!("{}{}\n", tabs, raw.text)),
            Stmt::Select(raw) => self.output.push_str(&format!("{}{}\n", tabs, raw.text)),
            Stmt::Raw(raw) => self.output.push_str(&format!("{}{}\n", tabs, raw.text)),
            Stmt::Expr(e) => {
                let text = if e.expr.has_try {
                    format!("{}?", e.expr.text)
                } else {
                    e.expr.text.clone()
                };
                self.output.push_str(&format!("{}{}\n", tabs, text));
            }
            Stmt::Match(m) => self.emit_match(m, tabs, indent),
            Stmt::If(i) => self.emit_if(i, tabs, indent),
        }
    }

    fn emit_match(&mut self, m: &MatchStmt, tabs: &str, indent: usize) {
        let value = if m.value.has_try {
            format!("{}?", m.value.text)
        } else {
            m.value.text.clone()
        };
        self.output
            .push_str(&format!("{}match {} {{\n", tabs, value));
        for arm in &m.arms {
            self.emit_match_arm(arm, tabs, indent);
        }
        self.output.push_str(&format!("{}}}\n", tabs));
    }

    fn emit_match_arm(&mut self, arm: &MatchArm, tabs: &str, indent: usize) {
        let inner_tabs = format!("{}\t", tabs);
        self.output.push_str(&inner_tabs);
        match &arm.pattern {
            Pattern::Wildcard { .. } => self.output.push('_'),
            Pattern::TypedVariant {
                enum_name,
                variant,
                bindings,
                ..
            } => {
                self.output.push_str(enum_name);
                self.output.push_str("::");
                self.output.push_str(variant);
                if !bindings.is_empty() {
                    self.output.push('(');
                    self.output.push_str(&bindings.join(", "));
                    self.output.push(')');
                }
            }
            Pattern::Variant {
                variant, bindings, ..
            } => {
                self.output.push_str(variant);
                if !bindings.is_empty() {
                    self.output.push('(');
                    self.output.push_str(&bindings.join(", "));
                    self.output.push(')');
                }
            }
        }
        self.output.push_str(" => ");
        match &arm.body {
            MatchArmBody::Expr(expr) => {
                let text = if expr.has_try {
                    format!("{}?", expr.text)
                } else {
                    expr.text.clone()
                };
                self.output.push_str(&text);
                self.output.push('\n');
            }
            MatchArmBody::Block(block) => {
                self.output.push_str("{\n");
                self.emit_block(block, indent + 2);
                self.output.push_str(&format!("{}}}\n", inner_tabs));
            }
        }
    }

    fn emit_if(&mut self, if_stmt: &IfStmt, tabs: &str, indent: usize) {
        let cond = if if_stmt.condition.has_try {
            format!("{}?", if_stmt.condition.text)
        } else {
            if_stmt.condition.text.clone()
        };
        self.output.push_str(&format!("{}if {} {{\n", tabs, cond));
        self.emit_block(&if_stmt.then_block, indent + 1);
        self.output.push_str(&format!("{}}}", tabs));
        if let Some(else_branch) = &if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => {
                    self.output.push_str(" else {\n");
                    self.emit_block(block, indent + 1);
                    self.output.push_str(&format!("{}}}\n", tabs));
                }
                ElseBranch::If(nested) => {
                    self.output.push_str(" else ");
                    self.emit_if(nested, "", indent);
                }
            }
        } else {
            self.output.push('\n');
        }
    }

    fn emit_raw(&mut self, decl: &RawDecl) {
        self.output.push_str(&decl.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;

    #[test]
    fn format_round_trip_simple() {
        let input = "package main\n\nimport \"fmt\"\n\nfn main() {\n\tfmt.Println(\"hello\")\n}\n";
        let program = parse_program(input).expect("parse");
        let formatted = format_gp(&program, input);
        assert!(formatted.contains("package main"));
        assert!(formatted.contains("import \"fmt\""));
        assert!(formatted.contains("fn main()"));
    }

    #[test]
    fn format_enum_with_derive() {
        let input = r#"package main

@derive(String)
enum Status {
    Pending
    Running
    Done
}
"#;
        let program = parse_program(input).expect("parse");
        let formatted = format_gp(&program, input);
        assert!(formatted.contains("@derive(String)"));
        assert!(formatted.contains("enum Status {"));
        assert!(formatted.contains("\tPending"));
    }

    #[test]
    fn format_decorated_function() {
        let input = r#"package main

import "fmt"

@log
@retry(3, 100)
fn readName() -> string! {
    return "goplus"
}
"#;
        let program = parse_program(input).expect("parse");
        let formatted = format_gp(&program, input);
        assert!(formatted.contains("@log"));
        assert!(formatted.contains("@retry(3, 100)"));
        assert!(formatted.contains("fn readName() -> string!"));
    }
}
