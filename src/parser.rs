use std::ops::Range;

use crate::{
    ast::*,
    diag::Diagnostic,
    lexer::{Token, TokenKind, lex},
};

pub fn parse_program(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    let mut parser = Parser::new(source, tokens);
    parser.parse_program()
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    idx: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            idx: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        self.skip_separators();
        self.expect(TokenKind::Package, "expected `package` declaration");
        let package_name = self
            .parse_ident("expected package name")
            .unwrap_or_else(|| "main".to_string());
        self.skip_separators();

        let mut imports = Vec::new();
        while self.at(TokenKind::Import) {
            if let Some(path) = self.parse_import_decl() {
                imports.push(path);
            }
            self.skip_separators();
        }

        let mut items = Vec::new();
        while !self.is_eof() {
            self.skip_separators();
            if self.is_eof() {
                break;
            }

            let annotations = self.parse_annotations();
            self.skip_separators();

            let item = if self.at(TokenKind::Fn) {
                self.parse_fn_decl(annotations).map(Item::Function)
            } else if self.at(TokenKind::Struct) {
                self.parse_struct_decl(annotations).map(Item::Struct)
            } else if self.at(TokenKind::Enum) {
                self.parse_enum_decl(annotations).map(Item::Enum)
            } else if self.at(TokenKind::Impl) {
                if !annotations.is_empty() {
                    for ann in annotations {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "decorators are not supported on `impl` blocks",
                                Some(ann.span),
                            )
                            .with_hint("attach decorators on methods instead"),
                        );
                    }
                }
                self.parse_impl_block().map(Item::Impl)
            } else {
                self.diagnostics.push(Diagnostic::new(
                    "expected top-level declaration",
                    Some(self.current_span()),
                ));
                self.idx += 1;
                None
            };

            if let Some(item) = item {
                items.push(item);
            } else {
                self.synchronize_top_level();
            }

            self.skip_separators();
        }

        if self.diagnostics.is_empty() {
            Ok(Program {
                package: package_name,
                imports,
                items,
                span: 0..self.source.len(),
            })
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    fn parse_import_decl(&mut self) -> Option<String> {
        self.expect(TokenKind::Import, "expected `import`");
        let token = self.expect_token(TokenKind::StringLit, "expected import path string")?;
        Some(self.token_text(&token).trim_matches('"').to_string())
    }

    fn parse_annotations(&mut self) -> Vec<Decorator> {
        let mut decorators = Vec::new();
        while self.at(TokenKind::At) {
            if let Some(decorator) = self.parse_annotation() {
                decorators.push(decorator);
            } else {
                break;
            }
            self.skip_separators();
        }
        decorators
    }

    fn parse_annotation(&mut self) -> Option<Decorator> {
        let at = self.expect_token(TokenKind::At, "expected `@`")?;
        let name = self.parse_ident("expected decorator name")?;
        let mut args = Vec::new();
        let mut end = at.span.end;

        if self.consume(TokenKind::LParen) {
            self.skip_separators();
            while !self.at(TokenKind::RParen) && !self.is_eof() {
                if let Some((arg, _, _)) =
                    self.parse_text_segment(&[TokenKind::Comma, TokenKind::RParen], true, false)
                {
                    args.push(arg);
                }
                self.skip_separators();
                if self.consume(TokenKind::Comma) {
                    self.skip_separators();
                    continue;
                }
                break;
            }
            if let Some(rp) = self.expect_token(TokenKind::RParen, "expected `)` after args") {
                end = rp.span.end;
            }
        }

        Some(Decorator {
            name,
            args,
            span: at.span.start..end,
        })
    }

    fn parse_struct_decl(&mut self, annotations: Vec<Decorator>) -> Option<StructDecl> {
        let start = self
            .expect_token(TokenKind::Struct, "expected `struct`")?
            .span
            .start;
        let name = self.parse_ident("expected struct name")?;
        let derives = self.extract_derives(&annotations);
        self.skip_separators();
        self.expect(TokenKind::LBrace, "expected `{` after struct name");

        let mut fields = Vec::new();
        self.skip_separators();
        while !self.at(TokenKind::RBrace) && !self.is_eof() {
            let fstart = self.current_span().start;
            let field_name = self.parse_ident("expected field name")?;
            self.expect(TokenKind::Colon, "expected `:` after field name");
            self.skip_separators();
            let (ty_raw, ty_span, _) = self.parse_text_segment(
                &[TokenKind::Comma, TokenKind::Newline, TokenKind::RBrace],
                true,
                true,
            )?;
            fields.push(FieldDecl {
                name: field_name,
                ty: TypeRef {
                    raw: ty_raw,
                    span: ty_span.clone(),
                },
                span: fstart..ty_span.end,
            });
            self.skip_separators();
            self.consume(TokenKind::Comma);
            self.skip_separators();
        }

        let end = self.expect_token(TokenKind::RBrace, "expected `}` after struct body");
        Some(StructDecl {
            name,
            fields,
            derives,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }

    fn parse_enum_decl(&mut self, annotations: Vec<Decorator>) -> Option<EnumDecl> {
        let start = self
            .expect_token(TokenKind::Enum, "expected `enum`")?
            .span
            .start;
        let name = self.parse_ident("expected enum name")?;
        let type_params = self.parse_type_params();
        let derives = self.extract_derives(&annotations);
        self.skip_separators();
        self.expect(TokenKind::LBrace, "expected `{` after enum name");

        let mut variants = Vec::new();
        self.skip_separators();
        while !self.at(TokenKind::RBrace) && !self.is_eof() {
            let vstart = self.current_span().start;
            let variant_name = self.parse_ident("expected enum variant name")?;
            let mut payload = Vec::new();
            self.skip_separators();
            if self.consume(TokenKind::LParen) {
                self.skip_separators();
                while !self.at(TokenKind::RParen) && !self.is_eof() {
                    let (ty_raw, ty_span, _) = self.parse_text_segment(
                        &[TokenKind::Comma, TokenKind::RParen],
                        true,
                        false,
                    )?;
                    payload.push(TypeRef {
                        raw: ty_raw,
                        span: ty_span,
                    });
                    self.skip_separators();
                    if self.consume(TokenKind::Comma) {
                        self.skip_separators();
                        continue;
                    }
                    break;
                }
                self.expect(TokenKind::RParen, "expected `)` after variant payload");
            }
            let vend = self.previous_end().unwrap_or(vstart);
            variants.push(EnumVariant {
                name: variant_name,
                payload,
                span: vstart..vend,
            });
            self.skip_separators();
            self.consume(TokenKind::Comma);
            self.skip_separators();
        }

        let end = self.expect_token(TokenKind::RBrace, "expected `}` after enum body");
        Some(EnumDecl {
            name,
            type_params,
            variants,
            derives,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }

    fn parse_impl_block(&mut self) -> Option<ImplBlock> {
        let start = self
            .expect_token(TokenKind::Impl, "expected `impl`")?
            .span
            .start;
        self.skip_separators();
        let (target, _, _) = self.parse_text_segment(&[TokenKind::LBrace], true, false)?;
        self.expect(TokenKind::LBrace, "expected `{` after impl target");

        let mut methods = Vec::new();
        self.skip_separators();
        while !self.at(TokenKind::RBrace) && !self.is_eof() {
            let decorators = self.parse_annotations();
            self.skip_separators();
            if !self.at(TokenKind::Fn) {
                self.diagnostics.push(Diagnostic::new(
                    "expected method declaration inside impl",
                    Some(self.current_span()),
                ));
                self.idx += 1;
                continue;
            }
            if let Some(method) = self.parse_method_decl(decorators) {
                methods.push(method);
            } else {
                self.synchronize_block();
            }
            self.skip_separators();
        }

        let end = self.expect_token(TokenKind::RBrace, "expected `}` after impl block");
        Some(ImplBlock {
            target,
            methods,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }
    fn parse_fn_decl(&mut self, decorators: Vec<Decorator>) -> Option<FnDecl> {
        let start = self
            .expect_token(TokenKind::Fn, "expected `fn`")?
            .span
            .start;
        let name = self.parse_ident("expected function name")?;
        let type_params = self.parse_type_params();
        let (params, _) = self.parse_params(false)?;
        let ret = self.parse_return_type();
        let body = self.parse_block()?;
        let end = body.span.end;
        Some(FnDecl {
            name,
            type_params,
            params,
            ret,
            body,
            decorators,
            span: start..end,
        })
    }

    fn parse_method_decl(&mut self, decorators: Vec<Decorator>) -> Option<MethodDecl> {
        let start = self
            .expect_token(TokenKind::Fn, "expected `fn`")?
            .span
            .start;
        let name = self.parse_ident("expected method name")?;
        let type_params = self.parse_type_params();
        let (params, receiver) = self.parse_params(true)?;
        let receiver = receiver.unwrap_or_else(|| {
            self.diagnostics.push(
                Diagnostic::new(
                    "expected `self` or `mut self` as first method argument",
                    Some(start..start),
                )
                .with_hint("use `fn m(self, ...)` or `fn m(mut self, ...)`"),
            );
            ReceiverKind::Value
        });
        let ret = self.parse_return_type();
        let body = self.parse_block()?;
        let end = body.span.end;
        Some(MethodDecl {
            receiver,
            name,
            type_params,
            params,
            ret,
            body,
            decorators,
            span: start..end,
        })
    }

    fn parse_params(
        &mut self,
        allow_receiver: bool,
    ) -> Option<(Vec<ParamDecl>, Option<ReceiverKind>)> {
        self.expect(TokenKind::LParen, "expected `(`");
        self.skip_separators();

        let mut receiver = None;
        let mut params = Vec::new();

        if allow_receiver {
            if self.at(TokenKind::SelfKw) {
                receiver = Some(ReceiverKind::Value);
                self.idx += 1;
                self.skip_separators();
                self.consume(TokenKind::Comma);
                self.skip_separators();
            } else if self.at(TokenKind::Mut) && self.at_n(1, TokenKind::SelfKw) {
                receiver = Some(ReceiverKind::Pointer);
                self.idx += 2;
                self.skip_separators();
                self.consume(TokenKind::Comma);
                self.skip_separators();
            }
        }

        while !self.at(TokenKind::RParen) && !self.is_eof() {
            let pstart = self.current_span().start;
            let name = self.parse_ident("expected parameter name")?;
            self.expect(TokenKind::Colon, "expected `:` after parameter name");
            self.skip_separators();
            let (ty_raw, ty_span, _) =
                self.parse_text_segment(&[TokenKind::Comma, TokenKind::RParen], true, false)?;
            params.push(ParamDecl {
                name,
                ty: TypeRef {
                    raw: ty_raw,
                    span: ty_span.clone(),
                },
                span: pstart..ty_span.end,
            });
            self.skip_separators();
            if self.consume(TokenKind::Comma) {
                self.skip_separators();
                continue;
            }
            break;
        }

        self.expect(TokenKind::RParen, "expected `)` after parameter list");
        Some((params, receiver))
    }

    fn parse_return_type(&mut self) -> ReturnType {
        self.skip_separators();
        if !self.consume(TokenKind::Arrow) {
            return ReturnType::Void;
        }
        self.skip_separators();

        let start_idx = self.idx;
        let mut i = self.idx;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut angle_depth = 0usize;

        while i < self.tokens.len() {
            let kind = self.tokens[i].kind;
            if paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && angle_depth == 0
                && kind == TokenKind::LBrace
            {
                break;
            }
            match kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Lt => angle_depth += 1,
                TokenKind::Gt => angle_depth = angle_depth.saturating_sub(1),
                _ => {}
            }
            i += 1;
        }

        if i <= start_idx {
            self.diagnostics.push(Diagnostic::new(
                "expected return type after `->`",
                Some(self.current_span()),
            ));
            return ReturnType::Void;
        }

        let segment = &self.tokens[start_idx..i];
        self.idx = i;

        if segment.len() == 1 && segment[0].kind == TokenKind::Bang {
            return ReturnType::ErrorOnly;
        }

        let has_error = segment
            .last()
            .map(|t| t.kind == TokenKind::Bang)
            .unwrap_or(false);

        let end_token_idx = if has_error { i.saturating_sub(1) } else { i };
        let span = self.range_span(start_idx, end_token_idx).unwrap_or(0..0);
        let ty = TypeRef {
            raw: self.source[span.clone()].trim().to_string(),
            span,
        };

        if has_error {
            ReturnType::TypeWithError(ty)
        } else {
            ReturnType::Type(ty)
        }
    }

    fn parse_type_params(&mut self) -> Vec<String> {
        self.skip_separators();
        if !self.consume(TokenKind::Lt) {
            return Vec::new();
        }
        let mut params = Vec::new();
        self.skip_separators();
        while !self.at(TokenKind::Gt) && !self.is_eof() {
            if let Some(name) = self.parse_ident("expected generic type parameter") {
                params.push(name);
            }
            self.skip_separators();
            if self.consume(TokenKind::Comma) {
                self.skip_separators();
                continue;
            }
            break;
        }
        self.expect(TokenKind::Gt, "expected `>` after type parameters");
        params
    }

    fn parse_block(&mut self) -> Option<Block> {
        let lb = self.expect_token(TokenKind::LBrace, "expected `{` to start block")?;
        let start = lb.span.start;
        let mut stmts = Vec::new();
        self.skip_separators();

        while !self.at(TokenKind::RBrace) && !self.is_eof() {
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.synchronize_block();
            }
            self.skip_separators();
        }

        let rb = self.expect_token(TokenKind::RBrace, "expected `}` to end block");
        let end = rb.map(|t| t.span.end).unwrap_or(start);
        Some(Block {
            stmts,
            span: start..end,
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        self.skip_separators();
        if self.is_eof() || self.at(TokenKind::RBrace) {
            return None;
        }

        if self.at(TokenKind::Match) {
            return self.parse_match_stmt().map(Stmt::Match);
        }
        if self.at(TokenKind::Return) {
            return self.parse_return_stmt().map(Stmt::Return);
        }
        if self.at(TokenKind::If) {
            return self.parse_if_stmt().map(Stmt::If);
        }
        if self.at(TokenKind::For) {
            return self.parse_raw_stmt().map(Stmt::Raw);
        }
        if self.at(TokenKind::Ident) && self.at_n(1, TokenKind::ColonEq) {
            return self.parse_var_decl_stmt().map(Stmt::VarDecl);
        }
        self.parse_expr_stmt().map(Stmt::Expr)
    }

    fn parse_var_decl_stmt(&mut self) -> Option<VarDeclStmt> {
        let start = self.current_span().start;
        let name = self.parse_ident("expected variable name")?;
        self.expect(TokenKind::ColonEq, "expected `:=` in variable declaration");
        self.skip_separators();
        let (text, span, has_try) = self
            .parse_text_segment(&[], true, true)
            .or_else(|| self.parse_raw_text_to_line())?;
        Some(VarDeclStmt {
            name,
            expr: Expr {
                text,
                has_try,
                span: span.clone(),
            },
            span: start..span.end,
        })
    }

    fn parse_return_stmt(&mut self) -> Option<ReturnStmt> {
        let start = self
            .expect_token(TokenKind::Return, "expected `return`")?
            .span
            .start;
        self.skip_separators();
        if self.at(TokenKind::Newline) || self.at(TokenKind::Semi) || self.at(TokenKind::RBrace) {
            return Some(ReturnStmt {
                exprs: Vec::new(),
                span: start..start,
            });
        }

        let mut exprs = Vec::new();
        while !self.is_eof() {
            let (text, span, has_try) = self.parse_text_segment(&[TokenKind::Comma], true, true)?;
            exprs.push(Expr {
                text,
                has_try,
                span,
            });
            self.skip_separators();
            if self.consume(TokenKind::Comma) {
                self.skip_separators();
                continue;
            }
            break;
        }

        let end = exprs.last().map(|e| e.span.end).unwrap_or(start);
        Some(ReturnStmt {
            exprs,
            span: start..end,
        })
    }

    fn parse_expr_stmt(&mut self) -> Option<ExprStmt> {
        let (text, span, has_try) = self
            .parse_text_segment(&[], true, true)
            .or_else(|| self.parse_raw_text_to_line())?;
        Some(ExprStmt {
            expr: Expr {
                text,
                has_try,
                span: span.clone(),
            },
            span,
        })
    }
    fn parse_match_stmt(&mut self) -> Option<MatchStmt> {
        let start = self
            .expect_token(TokenKind::Match, "expected `match`")?
            .span
            .start;
        self.skip_separators();
        let (value_text, value_span, value_try) =
            self.parse_text_segment(&[TokenKind::LBrace], true, false)?;
        self.expect(TokenKind::LBrace, "expected `{` after match value");
        self.skip_separators();

        let mut arms = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_eof() {
            self.skip_separators();
            if self.at(TokenKind::RBrace) {
                break;
            }

            let (pattern_text, pattern_span, _) =
                self.parse_text_segment(&[TokenKind::FatArrow], true, false)?;
            self.expect(TokenKind::FatArrow, "expected `=>` in match arm");
            self.skip_separators();

            let pattern = self.parse_pattern(&pattern_text, pattern_span.clone());
            let arm_body = if self.at(TokenKind::LBrace) {
                MatchArmBody::Block(self.parse_block()?)
            } else {
                let (expr_text, expr_span, expr_try) =
                    self.parse_text_segment(&[TokenKind::Comma], true, true)?;
                MatchArmBody::Expr(Expr {
                    text: expr_text,
                    has_try: expr_try,
                    span: expr_span,
                })
            };

            let arm_end = match &arm_body {
                MatchArmBody::Expr(expr) => expr.span.end,
                MatchArmBody::Block(block) => block.span.end,
            };
            arms.push(MatchArm {
                pattern,
                body: arm_body,
                span: pattern_span.start..arm_end,
            });

            self.skip_separators();
            self.consume(TokenKind::Comma);
            self.skip_separators();
        }

        let rb = self.expect_token(TokenKind::RBrace, "expected `}` after match arms");
        let end = rb.map(|t| t.span.end).unwrap_or(start);
        Some(MatchStmt {
            value: Expr {
                text: value_text,
                has_try: value_try,
                span: value_span,
            },
            arms,
            resolved_enum: None,
            span: start..end,
        })
    }

    fn parse_if_stmt(&mut self) -> Option<IfStmt> {
        let start = self
            .expect_token(TokenKind::If, "expected `if`")?
            .span
            .start;
        self.skip_separators();
        let (cond_text, cond_span, cond_try) =
            self.parse_text_segment(&[TokenKind::LBrace], true, false)?;
        let then_block = self.parse_block()?;
        self.skip_separators();
        let else_branch = if self.consume(TokenKind::Else) {
            self.skip_separators();
            if self.at(TokenKind::If) {
                Some(ElseBranch::If(Box::new(self.parse_if_stmt()?)))
            } else {
                Some(ElseBranch::Block(self.parse_block()?))
            }
        } else {
            None
        };

        let end = else_branch
            .as_ref()
            .map(|branch| match branch {
                ElseBranch::Block(block) => block.span.end,
                ElseBranch::If(if_stmt) => if_stmt.span.end,
            })
            .unwrap_or(then_block.span.end);

        Some(IfStmt {
            condition: Expr {
                text: cond_text,
                has_try: cond_try,
                span: cond_span,
            },
            then_block,
            else_branch,
            span: start..end,
        })
    }

    fn parse_raw_stmt(&mut self) -> Option<RawStmt> {
        let start_idx = self.idx;
        let mut i = self.idx;
        let mut brace_depth = 0usize;
        let mut seen_block = false;

        while i < self.tokens.len() {
            let kind = self.tokens[i].kind;
            match kind {
                TokenKind::LBrace => {
                    seen_block = true;
                    brace_depth += 1;
                }
                TokenKind::RBrace => {
                    if seen_block {
                        brace_depth = brace_depth.saturating_sub(1);
                        if brace_depth == 0 {
                            i += 1;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                TokenKind::Newline | TokenKind::Semi if !seen_block => break,
                _ => {}
            }
            i += 1;
        }

        if i == start_idx {
            return None;
        }
        let span = self.range_span(start_idx, i)?;
        let text = self.source[span.clone()].trim().to_string();
        self.idx = i;
        Some(RawStmt { text, span })
    }

    fn parse_pattern(&mut self, raw: &str, span: Span) -> Pattern {
        let text = raw.trim();
        if text == "_" {
            return Pattern::Wildcard { span };
        }

        if let Some((left, right)) = text.split_once("::") {
            let enum_name = left.trim().to_string();
            let (variant, bindings) = parse_pattern_variant(right.trim());
            return Pattern::TypedVariant {
                enum_name,
                variant,
                bindings,
                span,
            };
        }

        let (variant, bindings) = parse_pattern_variant(text);
        Pattern::Variant {
            variant,
            bindings,
            span,
        }
    }

    fn parse_raw_text_to_line(&mut self) -> Option<(String, Span, bool)> {
        self.parse_text_segment(&[], true, true)
    }

    fn parse_text_segment(
        &mut self,
        stop_kinds: &[TokenKind],
        stop_on_newline: bool,
        stop_on_rbrace: bool,
    ) -> Option<(String, Span, bool)> {
        let start_idx = self.idx;
        let mut i = self.idx;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut angle_depth = 0usize;

        while i < self.tokens.len() {
            let kind = self.tokens[i].kind;
            if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 && angle_depth == 0 {
                if stop_on_newline && kind == TokenKind::Newline {
                    break;
                }
                if stop_on_rbrace && kind == TokenKind::RBrace {
                    break;
                }
                if stop_kinds.contains(&kind) {
                    break;
                }
            }
            match kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Lt => angle_depth += 1,
                TokenKind::Gt => angle_depth = angle_depth.saturating_sub(1),
                _ => {}
            }
            i += 1;
        }

        if i <= start_idx {
            return None;
        }

        let mut expr_end_idx = i;
        while expr_end_idx > start_idx && self.tokens[expr_end_idx - 1].kind == TokenKind::Newline {
            expr_end_idx -= 1;
        }
        while expr_end_idx > start_idx && self.tokens[expr_end_idx - 1].kind == TokenKind::Semi {
            expr_end_idx -= 1;
        }
        if expr_end_idx <= start_idx {
            return None;
        }

        let mut has_try = false;
        if self.tokens[expr_end_idx - 1].kind == TokenKind::Question {
            has_try = true;
            expr_end_idx -= 1;
        }
        if expr_end_idx <= start_idx {
            self.diagnostics.push(Diagnostic::new(
                "expected expression before `?`",
                self.range_span(start_idx, i),
            ));
            self.idx = i;
            return None;
        }

        let expr_span = self.range_span(start_idx, expr_end_idx)?;
        let text = self.source[expr_span.clone()].trim().to_string();
        self.idx = i;
        Some((text, expr_span, has_try))
    }

    fn extract_derives(&mut self, annotations: &[Decorator]) -> Vec<DeriveKind> {
        let mut derives = Vec::new();
        for ann in annotations {
            if ann.name != "derive" {
                self.diagnostics.push(
                    Diagnostic::new(
                        format!("unsupported annotation `@{}` on type declaration", ann.name),
                        Some(ann.span.clone()),
                    )
                    .with_hint("only `@derive(String)` is supported on structs/enums"),
                );
                continue;
            }
            if ann.args.len() != 1 || ann.args[0].trim() != "String" {
                self.diagnostics.push(
                    Diagnostic::new("unsupported derive target", Some(ann.span.clone()))
                        .with_hint("goplus v1 supports only `@derive(String)`"),
                );
                continue;
            }
            derives.push(DeriveKind::String);
        }
        derives
    }

    fn synchronize_top_level(&mut self) {
        while !self.is_eof() {
            if self.at(TokenKind::Fn)
                || self.at(TokenKind::Struct)
                || self.at(TokenKind::Enum)
                || self.at(TokenKind::Impl)
            {
                break;
            }
            self.idx += 1;
        }
    }

    fn synchronize_block(&mut self) {
        let mut depth = 0usize;
        while !self.is_eof() {
            match self.peek_kind() {
                Some(TokenKind::LBrace) => {
                    depth += 1;
                    self.idx += 1;
                }
                Some(TokenKind::RBrace) => {
                    if depth == 0 {
                        break;
                    }
                    depth = depth.saturating_sub(1);
                    self.idx += 1;
                }
                Some(TokenKind::Newline | TokenKind::Semi) if depth == 0 => {
                    self.idx += 1;
                    break;
                }
                Some(_) => self.idx += 1,
                None => break,
            }
        }
    }

    fn skip_separators(&mut self) {
        while self.consume(TokenKind::Newline) || self.consume(TokenKind::Semi) {}
    }

    fn parse_ident(&mut self, message: &str) -> Option<String> {
        let token = self.expect_token(TokenKind::Ident, message)?;
        Some(self.token_text(&token).to_string())
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> bool {
        if self.consume(kind) {
            true
        } else {
            self.diagnostics
                .push(Diagnostic::new(message, Some(self.current_span())));
            false
        }
    }

    fn expect_token(&mut self, kind: TokenKind, message: &str) -> Option<Token> {
        if self.at(kind) {
            let token = self.tokens[self.idx].clone();
            self.idx += 1;
            Some(token)
        } else {
            self.diagnostics
                .push(Diagnostic::new(message, Some(self.current_span())));
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek_kind() == Some(kind)
    }

    fn at_n(&self, n: usize, kind: TokenKind) -> bool {
        self.tokens.get(self.idx + n).map(|t| t.kind) == Some(kind)
    }

    fn consume(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.idx += 1;
            true
        } else {
            false
        }
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.idx).map(|t| t.kind)
    }

    fn is_eof(&self) -> bool {
        self.idx >= self.tokens.len()
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.idx)
            .map(|t| t.span.clone())
            .unwrap_or_else(|| self.source.len()..self.source.len())
    }

    fn previous_end(&self) -> Option<usize> {
        self.idx
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map(|t| t.span.end)
    }

    fn token_text(&self, token: &Token) -> &str {
        &self.source[token.span.clone()]
    }

    fn range_span(&self, start_idx: usize, end_idx: usize) -> Option<Range<usize>> {
        if end_idx <= start_idx || start_idx >= self.tokens.len() {
            return None;
        }
        let start = self.tokens[start_idx].span.start;
        let end = self.tokens[end_idx.saturating_sub(1)].span.end;
        Some(start..end)
    }
}

fn parse_pattern_variant(input: &str) -> (String, Vec<String>) {
    let text = input.trim();
    if let Some(lp) = text.find('(') {
        if text.ends_with(')') {
            let name = text[..lp].trim().to_string();
            let inside = &text[lp + 1..text.len() - 1];
            let bindings = inside
                .split(',')
                .map(str::trim)
                .filter(|it| !it.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            return (name, bindings);
        }
    }
    (text.to_string(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::parse_program;
    use crate::ast::{Item, Pattern, ReturnType, Stmt};

    #[test]
    fn parse_decorated_function() {
        let src = r#"
package main

@log
@retry(3, 100)
fn load(path: string) -> string! {
    value := read(path)?
    return value
}
"#;
        let program = parse_program(src).expect("parse should succeed");
        let fn_decl = match &program.items[0] {
            Item::Function(it) => it,
            _ => panic!("expected function"),
        };
        assert_eq!(fn_decl.decorators.len(), 2);
        assert!(matches!(fn_decl.ret, ReturnType::TypeWithError(_)));
        match &fn_decl.body.stmts[0] {
            Stmt::VarDecl(stmt) => assert!(stmt.expr.has_try),
            _ => panic!("expected var decl"),
        }
    }

    #[test]
    fn parse_match_arm_patterns() {
        let src = r#"
package main

enum Result<T, E> {
    Ok(T)
    Err(E)
}

fn show(r: Result<int, string>) -> string {
    match r {
        Ok(v) => "ok",
        Err(e) => "err",
    }
}
"#;
        let program = parse_program(src).expect("parse should succeed");
        let fn_decl = match &program.items[1] {
            Item::Function(it) => it,
            _ => panic!("expected function"),
        };
        let match_stmt = match &fn_decl.body.stmts[0] {
            Stmt::Match(stmt) => stmt,
            _ => panic!("expected match"),
        };
        assert!(matches!(
            match_stmt.arms[0].pattern,
            Pattern::Variant { .. }
        ));
    }
}
