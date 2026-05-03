use super::*;

impl<'a> Parser<'a> {
    pub(super) fn extract_derives(&mut self, annotations: &[Decorator]) -> Vec<DeriveKind> {
        let mut derives = Vec::new();
        for ann in annotations {
            if ann.name != "derive" {
                self.diagnostics.push(
                    Diagnostic::new(
                        format!("unsupported annotation `@{}` on type declaration", ann.name),
                        Some(ann.span.clone()),
                    )
                    .with_hint("only `@derive(...)` is supported on structs/enums"),
                );
                continue;
            }
            if ann.args.is_empty() {
                self.diagnostics.push(
                    Diagnostic::new(
                        "@derive requires at least one target",
                        Some(ann.span.clone()),
                    )
                    .with_hint("example: @derive(String), @derive(Debug, Equal)"),
                );
                continue;
            }
            for arg in &ann.args {
                match arg.trim() {
                    "String" => derives.push(DeriveKind::String),
                    "Debug" => derives.push(DeriveKind::Debug),
                    "Equal" => derives.push(DeriveKind::Equal),
                    "JSON" => {
                        derives.push(DeriveKind::JsonMarshal);
                        derives.push(DeriveKind::JsonUnmarshal);
                    }
                    "JsonMarshal" => derives.push(DeriveKind::JsonMarshal),
                    "JsonUnmarshal" => derives.push(DeriveKind::JsonUnmarshal),
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                format!("unsupported derive target `{other}`"),
                                Some(ann.span.clone()),
                            )
                            .with_hint(
                                "supported: String, Debug, Equal, JSON, JsonMarshal, JsonUnmarshal",
                            ),
                        );
                    }
                }
            }
        }
        derives
    }

    pub(super) fn synchronize_top_level(&mut self) {
        while !self.is_eof() {
            if self.at(TokenKind::Fn)
                || self.at(TokenKind::Func)
                || self.at(TokenKind::Struct)
                || self.at(TokenKind::Enum)
                || self.at(TokenKind::Impl)
            {
                break;
            }
            self.idx += 1;
        }
    }

    pub(super) fn synchronize_block(&mut self) {
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

    pub(super) fn skip_separators(&mut self) {
        while self.consume(TokenKind::Newline) || self.consume(TokenKind::Semi) {}
    }

    pub(super) fn parse_ident(&mut self, message: &str) -> Option<String> {
        let token = self.expect_token(TokenKind::Ident, message)?;
        Some(self.token_text(&token).to_string())
    }

    pub(super) fn expect(&mut self, kind: TokenKind, message: &str) -> bool {
        if self.consume(kind) {
            true
        } else {
            self.diagnostics.push(
                Diagnostic::new(message, Some(self.current_span()))
                    .with_code("E1001")
                    .with_hint("check the surrounding syntax and delimiter balance"),
            );
            false
        }
    }

    pub(super) fn expect_token(&mut self, kind: TokenKind, message: &str) -> Option<Token> {
        if self.at(kind) {
            let token = self.tokens[self.idx].clone();
            self.idx += 1;
            Some(token)
        } else {
            self.diagnostics.push(
                Diagnostic::new(message, Some(self.current_span()))
                    .with_code("E1001")
                    .with_hint("check the surrounding syntax and delimiter balance"),
            );
            None
        }
    }

    pub(super) fn at(&self, kind: TokenKind) -> bool {
        self.peek_kind() == Some(kind)
    }

    pub(super) fn at_n(&self, n: usize, kind: TokenKind) -> bool {
        self.tokens.get(self.idx + n).map(|t| t.kind) == Some(kind)
    }

    pub(super) fn consume(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.idx += 1;
            true
        } else {
            false
        }
    }

    pub(super) fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.idx).map(|t| t.kind)
    }

    pub(super) fn is_eof(&self) -> bool {
        self.idx >= self.tokens.len()
    }

    pub(super) fn current_span(&self) -> Span {
        self.tokens
            .get(self.idx)
            .map(|t| t.span.clone())
            .unwrap_or_else(|| self.source.len()..self.source.len())
    }

    pub(super) fn previous_end(&self) -> Option<usize> {
        self.idx
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map(|t| t.span.end)
    }

    pub(super) fn token_text(&self, token: &Token) -> &str {
        &self.source[token.span.clone()]
    }

    pub(super) fn range_span(&self, start_idx: usize, end_idx: usize) -> Option<Range<usize>> {
        if end_idx <= start_idx || start_idx >= self.tokens.len() {
            return None;
        }
        let start = self.tokens[start_idx].span.start;
        let end = self.tokens[end_idx.saturating_sub(1)].span.end;
        Some(start..end)
    }
}
