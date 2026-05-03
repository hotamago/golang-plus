use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_struct_decl(&mut self, annotations: Vec<Decorator>) -> Option<StructDecl> {
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
            let (ty_raw, ty_span, tag) = split_struct_field_type_and_tag(&ty_raw, &ty_span);
            let field_end = tag.as_ref().map(|tag| tag.span.end).unwrap_or(ty_span.end);
            fields.push(FieldDecl {
                name: field_name,
                ty: TypeRef {
                    raw: ty_raw,
                    span: ty_span.clone(),
                },
                tag: tag.map(|tag| tag.raw),
                span: fstart..field_end,
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
            source: None,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }

    pub(super) fn parse_enum_decl(&mut self, annotations: Vec<Decorator>) -> Option<EnumDecl> {
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
            source: None,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }

    pub(super) fn parse_impl_block(&mut self) -> Option<ImplBlock> {
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
            if !self.at(TokenKind::Fn) && !self.at(TokenKind::Func) {
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
            source: None,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }
    pub(super) fn parse_fn_decl(&mut self, decorators: Vec<Decorator>) -> Option<FnDecl> {
        let start = self.consume_fn_or_func()?.start;
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
            source: None,
            span: start..end,
        })
    }

    pub(super) fn parse_method_decl(&mut self, decorators: Vec<Decorator>) -> Option<MethodDecl> {
        let start = self.consume_fn_or_func()?.start;
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

    pub(super) fn parse_params(
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
            let mut names = vec![self.parse_ident("expected parameter name")?];
            // Support both GoPlus style `name: Type` and Go style `name Type`
            let has_colon = self.consume(TokenKind::Colon);
            if !has_colon {
                while self.at(TokenKind::Comma) && self.at_n(1, TokenKind::Ident) {
                    let next_kind = self.tokens.get(self.idx + 2).map(|token| token.kind);
                    if matches!(next_kind, Some(TokenKind::Colon | TokenKind::RParen)) {
                        break;
                    }
                    self.idx += 1;
                    names.push(self.parse_ident("expected parameter name")?);
                }
            }
            self.skip_separators();
            let (ty_raw, ty_span, _) =
                self.parse_text_segment(&[TokenKind::Comma, TokenKind::RParen], true, false)?;
            for name in names {
                params.push(ParamDecl {
                    name,
                    ty: TypeRef {
                        raw: ty_raw.clone(),
                        span: ty_span.clone(),
                    },
                    span: pstart..ty_span.end,
                });
            }
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

    pub(super) fn parse_return_type(&mut self) -> ReturnType {
        self.skip_separators();
        let has_arrow = self.consume(TokenKind::Arrow);
        if !has_arrow {
            // Go-style: check for return type without arrow
            // Could be: `func f() Type {`, `func f() (Type, error) {`, or no return type
            if self.at(TokenKind::LBrace) || self.is_eof() {
                return ReturnType::Void;
            }
            // Check for Go-style tuple return: `(Type, error)`
            if self.at(TokenKind::LParen) {
                return self.parse_go_tuple_return();
            }
            // Check for Go-style single return type: an Ident before `{`
            if self.at(TokenKind::Ident)
                || self.at(TokenKind::Func)
                || self.at(TokenKind::Star)
                || self.at(TokenKind::LBracket)
            {
                return self.parse_go_single_return();
            }
            // No return type
            return ReturnType::Void;
        }
        self.skip_separators();
        self.parse_arrow_return_type()
    }

    /// Parse return type after `->` (GoPlus-style)
    fn parse_arrow_return_type(&mut self) -> ReturnType {
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

    /// Parse Go-style tuple return: `(string, error)` or `(int, string)`
    fn parse_go_tuple_return(&mut self) -> ReturnType {
        let start_idx = self.idx;
        // Consume the entire (...) segment
        let mut i = self.idx;
        let mut paren_depth = 0usize;
        while i < self.tokens.len() {
            match self.tokens[i].kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => {
                    paren_depth = paren_depth.saturating_sub(1);
                    if paren_depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let span = self.range_span(start_idx, i).unwrap_or(0..0);
        let raw_text = self.source[span.clone()].trim().to_string();
        self.idx = i;

        // Check if this is a Go-style (T, error) pattern
        if let Some(inner) = raw_text.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<&str> = inner.splitn(2, ',').collect();
            if parts.len() == 2 && parts[1].trim() == "error" {
                let value_ty = parts[0].trim().to_string();
                return ReturnType::GoTypeWithError(TypeRef {
                    raw: value_ty,
                    span,
                });
            }
        }
        // Not a (T, error) pattern, treat as raw Go return type
        ReturnType::Type(TypeRef {
            raw: raw_text,
            span,
        })
    }

    /// Parse Go-style single return type: `string`, `int`, etc. before `{`
    fn parse_go_single_return(&mut self) -> ReturnType {
        let start_idx = self.idx;
        let mut i = self.idx;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        while i < self.tokens.len() {
            let kind = self.tokens[i].kind;
            if paren_depth == 0
                && bracket_depth == 0
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
                TokenKind::Lt => angle_depth += 1,
                TokenKind::Gt => angle_depth = angle_depth.saturating_sub(1),
                _ => {}
            }
            i += 1;
        }
        if i <= start_idx {
            return ReturnType::Void;
        }
        let span = self.range_span(start_idx, i).unwrap_or(0..0);
        let raw = self.source[span.clone()].trim().to_string();
        self.idx = i;
        if raw == "error" {
            return ReturnType::ErrorOnly;
        }
        ReturnType::Type(TypeRef { raw, span })
    }

    /// Consume either `fn` or `func` token, returning its span
    fn consume_fn_or_func(&mut self) -> Option<std::ops::Range<usize>> {
        if self.at(TokenKind::Fn) {
            let token = self.expect_token(TokenKind::Fn, "expected `fn` or `func`")?;
            Some(token.span)
        } else if self.at(TokenKind::Func) {
            let token = self.expect_token(TokenKind::Func, "expected `fn` or `func`")?;
            Some(token.span)
        } else {
            self.diagnostics.push(Diagnostic::new(
                "expected `fn` or `func`",
                Some(self.current_span()),
            ));
            None
        }
    }

    pub(super) fn parse_type_params(&mut self) -> Vec<String> {
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
}

struct ParsedFieldTag {
    raw: String,
    span: std::ops::Range<usize>,
}

fn split_struct_field_type_and_tag(
    raw: &str,
    span: &std::ops::Range<usize>,
) -> (String, std::ops::Range<usize>, Option<ParsedFieldTag>) {
    let trimmed_end = raw.trim_end().len();
    let trimmed = &raw[..trimmed_end];
    let Some(tag_end_rel) = trimmed.rfind('`') else {
        return (raw.trim().to_string(), span.clone(), None);
    };
    let Some(tag_start_rel) = trimmed[..tag_end_rel].rfind('`') else {
        return (raw.trim().to_string(), span.clone(), None);
    };
    if !trimmed[tag_end_rel + 1..].trim().is_empty() {
        return (raw.trim().to_string(), span.clone(), None);
    }

    let type_raw = trimmed[..tag_start_rel].trim_end();
    if type_raw.is_empty() {
        return (raw.trim().to_string(), span.clone(), None);
    }

    let leading_ws = raw.len().saturating_sub(raw.trim_start().len());
    let type_span = span.start + leading_ws..span.start + tag_start_rel;
    let tag_span = span.start + tag_start_rel..span.start + tag_end_rel + 1;
    let tag_raw = trimmed[tag_start_rel..=tag_end_rel].to_string();
    (
        type_raw.trim().to_string(),
        type_span,
        Some(ParsedFieldTag {
            raw: tag_raw,
            span: tag_span,
        }),
    )
}
