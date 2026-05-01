use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_raw_text_to_line(&mut self) -> Option<(String, Span, bool)> {
        self.parse_text_segment(&[], true, true)
    }

    pub(super) fn parse_text_segment(
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
}
