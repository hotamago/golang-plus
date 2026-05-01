use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        self.skip_separators();
        self.expect(TokenKind::Package, "expected `package` declaration");
        let package_name = self
            .parse_ident("expected package name")
            .unwrap_or_else(|| "main".to_string());
        self.skip_separators();

        let mut imports = Vec::new();
        while self.at(TokenKind::Import) {
            imports.extend(self.parse_import_decl());
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

    pub(super) fn parse_import_decl(&mut self) -> Vec<String> {
        self.expect(TokenKind::Import, "expected `import`");
        if self.consume(TokenKind::LParen) {
            let mut imports = Vec::new();
            self.skip_separators();
            while !self.at(TokenKind::RParen) && !self.is_eof() {
                if let Some(token) =
                    self.expect_token(TokenKind::StringLit, "expected import path string")
                {
                    imports.push(self.token_text(&token).trim_matches('"').to_string());
                }
                self.skip_separators();
                self.consume(TokenKind::Semi);
                self.skip_separators();
            }
            self.expect(TokenKind::RParen, "expected `)` after import group");
            return imports;
        }

        self.expect_token(TokenKind::StringLit, "expected import path string")
            .map(|token| vec![self.token_text(&token).trim_matches('"').to_string()])
            .unwrap_or_default()
    }

    pub(super) fn parse_annotations(&mut self) -> Vec<Decorator> {
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

    pub(super) fn parse_annotation(&mut self) -> Option<Decorator> {
        let at = self.expect_token(TokenKind::At, "expected `@`")?;
        let name = self.parse_decorator_name()?;
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

    pub(super) fn parse_decorator_name(&mut self) -> Option<String> {
        let mut name = self.parse_ident("expected decorator name")?;
        while self.consume(TokenKind::Dot) {
            let segment = self.parse_ident("expected decorator name segment after `.`")?;
            name.push('.');
            name.push_str(&segment);
        }
        Some(name)
    }
}
