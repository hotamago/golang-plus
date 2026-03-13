use std::fmt;

use crate::ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render(&self, path: &str, source: &str) -> String {
        match &self.span {
            Some(span) => {
                let (line, col) = line_col(source, span.start);
                match &self.hint {
                    Some(hint) => format!("{path}:{line}:{col}: {} ({hint})", self.message),
                    None => format!("{path}:{line}:{col}: {}", self.message),
                }
            }
            None => match &self.hint {
                Some(hint) => format!("{path}: {} ({hint})", self.message),
                None => format!("{path}: {}", self.message),
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.items.extend(diagnostics);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn items(&self) -> &[Diagnostic] {
        &self.items
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diag in &self.items {
            writeln!(f, "{}", diag.message)?;
        }
        Ok(())
    }
}

pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
