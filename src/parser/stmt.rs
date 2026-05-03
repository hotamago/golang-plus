use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_block(&mut self) -> Option<Block> {
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

    pub(super) fn parse_stmt(&mut self) -> Option<Stmt> {
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
            return self.parse_raw_stmt().map(Stmt::For);
        }
        if self.at(TokenKind::Switch) {
            return self.parse_raw_stmt().map(Stmt::Switch);
        }
        if self.at(TokenKind::Select) {
            return self.parse_raw_stmt().map(Stmt::Select);
        }
        if self.at(TokenKind::Defer) {
            return self.parse_raw_line_stmt().map(Stmt::Defer);
        }
        if self.at(TokenKind::Go) {
            return self.parse_raw_line_stmt().map(Stmt::Go);
        }
        if self.at(TokenKind::Ident) && self.at_n(1, TokenKind::ColonEq) {
            return self.parse_var_decl_stmt().map(Stmt::VarDecl);
        }
        if self.at(TokenKind::Ident) && self.at_n(1, TokenKind::Eq) {
            return self.parse_assign_stmt().map(Stmt::Assign);
        }
        self.parse_expr_stmt().map(Stmt::Expr)
    }

    pub(super) fn parse_var_decl_stmt(&mut self) -> Option<VarDeclStmt> {
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

    pub(super) fn parse_return_stmt(&mut self) -> Option<ReturnStmt> {
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

    pub(super) fn parse_assign_stmt(&mut self) -> Option<AssignStmt> {
        let raw = self.parse_raw_line_stmt()?;
        Some(AssignStmt {
            text: raw.text,
            span: raw.span,
        })
    }

    pub(super) fn parse_expr_stmt(&mut self) -> Option<ExprStmt> {
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
    pub(super) fn parse_match_stmt(&mut self) -> Option<MatchStmt> {
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

    pub(super) fn parse_if_stmt(&mut self) -> Option<IfStmt> {
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

    pub(super) fn parse_raw_stmt(&mut self) -> Option<RawStmt> {
        let start_idx = self.idx;
        let mut i = self.idx;
        let mut brace_depth = 0usize;
        let mut seen_block = false;
        let expects_block =
            self.at(TokenKind::For) || self.at(TokenKind::Switch) || self.at(TokenKind::Select);

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
                TokenKind::Newline | TokenKind::Semi if !seen_block && !expects_block => break,
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

    fn parse_raw_line_stmt(&mut self) -> Option<RawStmt> {
        let (text, span, _) = self.parse_raw_text_to_line()?;
        Some(RawStmt { text, span })
    }
}
