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
            source: None,
            span: start..end.map(|t| t.span.end).unwrap_or(start),
        })
    }
    pub(super) fn parse_fn_decl(&mut self, decorators: Vec<Decorator>) -> Option<FnDecl> {
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
            source: None,
            span: start..end,
        })
    }

    pub(super) fn parse_method_decl(&mut self, decorators: Vec<Decorator>) -> Option<MethodDecl> {
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

    pub(super) fn parse_return_type(&mut self) -> ReturnType {
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
