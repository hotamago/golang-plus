use std::fmt;

use crate::ast::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub span: Option<Span>,
    pub hint: Option<String>,
    pub severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code: "E0000".to_string(),
            message: message.into(),
            span,
            hint: None,
            severity: DiagnosticSeverity::Error,
        }
    }

    pub fn warning(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code: "W0000".to_string(),
            message: message.into(),
            span,
            hint: None,
            severity: DiagnosticSeverity::Warning,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn render(&self, path: &str, source: &str) -> String {
        match &self.span {
            Some(span) => {
                let (line, col) = line_col(source, span.start);
                let mut out = format!(
                    "{path}:{line}:{col}: {} {}: {}",
                    self.severity, self.code, self.message
                );
                if let Some(line_text) = source_line(source, line) {
                    out.push('\n');
                    out.push_str(line_text);
                    out.push('\n');
                    out.push_str(&caret_line(col, span_len(source, span)));
                }
                if let Some(hint) = &self.hint {
                    out.push('\n');
                    out.push_str("hint: ");
                    out.push_str(hint);
                }
                out
            }
            None => match &self.hint {
                Some(hint) => format!(
                    "{path}: {} {}: {} (hint: {hint})",
                    self.severity, self.code, self.message
                ),
                None => format!("{path}: {} {}: {}", self.severity, self.code, self.message),
            },
        }
    }

    pub fn render_json(&self, path: &str, source: &str) -> String {
        let (line, column) = self
            .span
            .as_ref()
            .map(|span| line_col(source, span.start))
            .unwrap_or((0, 0));
        let (end_line, end_column) = self
            .span
            .as_ref()
            .map(|span| line_col(source, span.end))
            .unwrap_or((0, 0));
        let span = self
            .span
            .as_ref()
            .map(|span| {
                format!(
                    r#"{{"start":{},"end":{},"line":{},"column":{},"endLine":{},"endColumn":{}}}"#,
                    span.start, span.end, line, column, end_line, end_column
                )
            })
            .unwrap_or_else(|| "null".to_string());
        let hint = self
            .hint
            .as_ref()
            .map(|hint| format!(r#""{}""#, json_escape(hint)))
            .unwrap_or_else(|| "null".to_string());
        format!(
            r#"{{"path":"{}","code":"{}","severity":"{}","message":"{}","span":{},"hint":{},"source":"goplus"}}"#,
            json_escape(path),
            json_escape(&self.code),
            self.severity.as_str(),
            json_escape(&self.message),
            span,
            hint
        )
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
        self.items
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn has_any(&self) -> bool {
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

fn source_line(source: &str, target_line: usize) -> Option<&str> {
    source.lines().nth(target_line.saturating_sub(1))
}

fn caret_line(col: usize, len: usize) -> String {
    let width = len.max(1);
    format!("{}{}", " ".repeat(col.saturating_sub(1)), "^".repeat(width))
}

fn span_len(source: &str, span: &Span) -> usize {
    let end = span.end.min(source.len());
    let start = span.start.min(end);
    source[start..end]
        .chars()
        .take_while(|ch| *ch != '\n')
        .count()
}

pub fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}
