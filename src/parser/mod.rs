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
}

mod decl;
mod expr;
mod pattern;
mod program;
mod recovery;
mod stmt;

#[cfg(test)]
mod tests;
