use logos::Logos;

use crate::diag::Diagnostic;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[ \t\r\f]+")]
#[logos(skip r"//[^\n]*")]
pub enum TokenKind {
    #[token("\n")]
    Newline,

    #[token("package")]
    Package,
    #[token("import")]
    Import,
    #[token("fn")]
    Fn,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("impl")]
    Impl,
    #[token("match")]
    Match,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("mut")]
    Mut,
    #[token("self")]
    SelfKw,
    #[token("type")]
    TypeKw,

    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("::")]
    DoubleColon,
    #[token(":=")]
    ColonEq,

    #[token("@")]
    At,
    #[token("!")]
    Bang,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("=")]
    Eq,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,

    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLit,
    #[regex(r"[0-9][0-9_]*")]
    IntLit,
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: std::ops::Range<usize>,
}

pub fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut lexer = TokenKind::lexer(source);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    while let Some(item) = lexer.next() {
        let span = lexer.span();
        match item {
            Ok(kind) => tokens.push(Token {
                kind,
                span: span.start..span.end,
            }),
            Err(_) => diagnostics.push(Diagnostic::new(
                format!("invalid token `{}`", &source[span.start..span.end]),
                Some(span.start..span.end),
            )),
        }
    }

    if diagnostics.is_empty() {
        Ok(tokens)
    } else {
        Err(diagnostics)
    }
}
