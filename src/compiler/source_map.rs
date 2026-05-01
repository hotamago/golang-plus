use std::{collections::BTreeSet, fs, path::Path};

use crate::ast::*;
use serde::Serialize;

use super::*;

#[derive(Debug, Clone, Serialize)]
pub(super) struct SourceMapFile {
    version: u8,
    generated: String,
    sources: Vec<String>,
    mappings: Vec<SourceMapMapping>,
}

#[derive(Debug, Clone, Serialize)]
struct SourceMapMapping {
    source: String,
    source_start: usize,
    source_end: usize,
    generated_start: usize,
    generated_end: usize,
    kind: String,
    name: String,
}

pub(super) fn build_source_map(project: &Project, generated_file: &Path) -> Result<SourceMapFile> {
    let generated = fs::read_to_string(generated_file).with_context(|| {
        format!(
            "failed to read generated file for source map {}",
            generated_file.display()
        )
    })?;
    let mut builder = SourceMapBuilder::new(generated_file, &generated);
    for unit in &project.units {
        builder.add_source(unit.path.display().to_string());
        for item in &unit.program.items {
            builder.map_item(item);
        }
    }
    Ok(builder.finish())
}

impl SourceMapFile {
    pub(super) fn to_json(&self) -> Result<String> {
        Ok(format!("{}\n", serde_json::to_string_pretty(self)?))
    }
}

struct SourceMapBuilder<'a> {
    generated_file: &'a Path,
    generated: &'a str,
    sources: BTreeSet<String>,
    mappings: Vec<SourceMapMapping>,
    cursor: usize,
}

impl<'a> SourceMapBuilder<'a> {
    fn new(generated_file: &'a Path, generated: &'a str) -> Self {
        Self {
            generated_file,
            generated,
            sources: BTreeSet::new(),
            mappings: Vec::new(),
            cursor: 0,
        }
    }

    fn add_source(&mut self, source: String) {
        self.sources.insert(source);
    }

    fn finish(self) -> SourceMapFile {
        SourceMapFile {
            version: 1,
            generated: self.generated_file.display().to_string(),
            sources: self.sources.into_iter().collect(),
            mappings: self.mappings,
        }
    }

    fn map_item(&mut self, item: &Item) {
        match item {
            Item::Struct(decl) => {
                self.map_named(
                    decl.source.as_deref(),
                    decl.span.clone(),
                    "declaration",
                    &decl.name,
                    &format!("type {} struct", decl.name),
                );
            }
            Item::Enum(decl) => {
                self.map_named(
                    decl.source.as_deref(),
                    decl.span.clone(),
                    "declaration",
                    &decl.name,
                    &format!("type {}", decl.name),
                );
            }
            Item::Function(decl) => self.map_function(decl),
            Item::Impl(decl) => {
                self.map_named(
                    decl.source.as_deref(),
                    decl.span.clone(),
                    "declaration",
                    &decl.target,
                    "func (",
                );
                for method in &decl.methods {
                    self.map_method(decl, method);
                }
            }
            Item::Raw(decl) => {
                self.map_named(
                    decl.source.as_deref(),
                    decl.span.clone(),
                    "declaration",
                    raw_decl_name(&decl.text),
                    first_search_line(&decl.text),
                );
            }
        }
    }

    fn map_function(&mut self, decl: &FnDecl) {
        let go_name = if decl.name == "main" && matches!(decl.ret, ReturnType::ErrorOnly) {
            "mainWarp"
        } else {
            decl.name.as_str()
        };
        let start = self.map_named(
            decl.source.as_deref(),
            decl.span.clone(),
            "function",
            &decl.name,
            &format!("func {go_name}"),
        );
        self.map_block(
            decl.source.as_deref(),
            &decl.body,
            start.unwrap_or(self.cursor),
        );
    }

    fn map_method(&mut self, impl_block: &ImplBlock, method: &MethodDecl) {
        let pattern = format!(") {}", method.name);
        let start = self.map_named(
            impl_block.source.as_deref(),
            method.span.clone(),
            "method",
            &format!("{}.{}", impl_block.target, method.name),
            &pattern,
        );
        self.map_block(
            impl_block.source.as_deref(),
            &method.body,
            start.unwrap_or(self.cursor),
        );
    }

    fn map_block(&mut self, source: Option<&str>, block: &Block, mut cursor: usize) {
        for stmt in &block.stmts {
            cursor = self.map_stmt(source, stmt, cursor).unwrap_or(cursor);
        }
    }

    fn map_stmt(&mut self, source: Option<&str>, stmt: &Stmt, cursor: usize) -> Option<usize> {
        match stmt {
            Stmt::VarDecl(stmt) => self.map_named_from(
                source,
                stmt.span.clone(),
                "statement",
                &stmt.name,
                &format!("{} := ", stmt.name),
                cursor,
            ),
            Stmt::Assign(stmt) => self.map_named_from(
                source,
                stmt.span.clone(),
                "statement",
                "assign",
                first_search_line(&stmt.text),
                cursor,
            ),
            Stmt::Return(stmt) => self.map_named_from(
                source,
                stmt.span.clone(),
                "statement",
                "return",
                "return",
                cursor,
            ),
            Stmt::Defer(raw) => self.map_raw_stmt(source, raw, "defer", cursor),
            Stmt::Go(raw) => self.map_raw_stmt(source, raw, "go", cursor),
            Stmt::Expr(stmt) => self.map_named_from(
                source,
                stmt.span.clone(),
                "statement",
                "expr",
                first_search_line(&stmt.expr.text),
                cursor,
            ),
            Stmt::Match(stmt) => {
                let next = self.map_named_from(
                    source,
                    stmt.span.clone(),
                    "statement",
                    "match",
                    "switch",
                    cursor,
                );
                let mut arm_cursor = next.unwrap_or(cursor);
                for arm in &stmt.arms {
                    arm_cursor = self
                        .map_match_arm(source, arm, arm_cursor)
                        .unwrap_or(arm_cursor);
                }
                next
            }
            Stmt::If(stmt) => {
                let next = self.map_named_from(
                    source,
                    stmt.span.clone(),
                    "statement",
                    "if",
                    "if ",
                    cursor,
                );
                self.map_block(source, &stmt.then_block, next.unwrap_or(cursor));
                if let Some(branch) = &stmt.else_branch {
                    match branch {
                        ElseBranch::Block(block) => {
                            self.map_block(source, block, next.unwrap_or(cursor))
                        }
                        ElseBranch::If(nested) => {
                            self.map_stmt(
                                source,
                                &Stmt::If((**nested).clone()),
                                next.unwrap_or(cursor),
                            );
                        }
                    }
                }
                next
            }
            Stmt::For(raw) => self.map_raw_stmt(source, raw, "for", cursor),
            Stmt::Switch(raw) => self.map_raw_stmt(source, raw, "switch", cursor),
            Stmt::Select(raw) => self.map_raw_stmt(source, raw, "select", cursor),
            Stmt::Raw(raw) => self.map_raw_stmt(source, raw, "raw", cursor),
        }
    }

    fn map_raw_stmt(
        &mut self,
        source: Option<&str>,
        raw: &RawStmt,
        name: &str,
        cursor: usize,
    ) -> Option<usize> {
        self.map_named_from(
            source,
            raw.span.clone(),
            "statement",
            name,
            first_search_line(&raw.text),
            cursor,
        )
    }

    fn map_match_arm(
        &mut self,
        source: Option<&str>,
        arm: &MatchArm,
        cursor: usize,
    ) -> Option<usize> {
        let pattern = match &arm.pattern {
            Pattern::Wildcard { .. } => "default:".to_string(),
            Pattern::TypedVariant { variant, .. } | Pattern::Variant { variant, .. } => {
                variant.clone()
            }
        };
        self.map_named_from(
            source,
            arm.span.clone(),
            "match_arm",
            arm.pattern.variant_name().unwrap_or("_"),
            &pattern,
            cursor,
        )
    }

    fn map_named(
        &mut self,
        source: Option<&str>,
        source_span: Span,
        kind: &str,
        name: &str,
        generated_pattern: &str,
    ) -> Option<usize> {
        self.map_named_from(
            source,
            source_span,
            kind,
            name,
            generated_pattern,
            self.cursor,
        )
    }

    fn map_named_from(
        &mut self,
        source: Option<&str>,
        source_span: Span,
        kind: &str,
        name: &str,
        generated_pattern: &str,
        cursor: usize,
    ) -> Option<usize> {
        let source = source?;
        let (start, end) = find_line_range(self.generated, generated_pattern, cursor)
            .or_else(|| find_line_range(self.generated, generated_pattern, 0))?;
        self.cursor = end;
        self.mappings.push(SourceMapMapping {
            source: source.to_string(),
            source_start: source_span.start,
            source_end: source_span.end,
            generated_start: start,
            generated_end: end,
            kind: kind.to_string(),
            name: name.to_string(),
        });
        Some(end)
    }
}

fn find_line_range(haystack: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let offset = haystack.get(from..)?.find(needle).map(|idx| from + idx)?;
    let line_start = haystack[..offset]
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let line_end = haystack[offset..]
        .find('\n')
        .map(|idx| offset + idx)
        .unwrap_or(haystack.len());
    Some((line_start, line_end))
}

fn first_search_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
}

fn raw_decl_name(text: &str) -> &str {
    text.split_whitespace().nth(1).unwrap_or("raw")
}
