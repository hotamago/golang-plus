use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::diag::line_col;

/// Location in a generated `.go` file.
#[derive(Debug, Clone)]
pub struct GoLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// Location in an original `.gp` source file.
#[derive(Debug, Clone)]
pub struct GpLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SourceMapFile {
    pub version: u8,
    pub generated: String,
    pub sources: Vec<String>,
    pub mappings: Vec<SourceMapMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SourceMapMapping {
    pub source: String,
    pub source_start: usize,
    pub source_end: usize,
    pub generated_start: usize,
    pub generated_end: usize,
    pub kind: String,
    pub name: String,
}

impl SourceMapFile {
    /// Load a source map from a JSON file.
    pub fn from_json_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read source map {}", path.display()))?;
        Self::from_json(&content)
    }

    /// Parse a source map from a JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("failed to parse source map JSON")
    }
}

/// Given a position in a `.gp` file, find the corresponding `.go` position.
pub fn lookup_gp_to_go(
    source_map: &SourceMapFile,
    gp_path: &str,
    gp_line: usize,
    gp_column: usize,
) -> Option<GoLocation> {
    // Read the source file to convert line:col to offset
    let source_content = fs::read_to_string(gp_path).ok()?;
    let offset = line_col_to_offset(&source_content, gp_line, gp_column)?;

    // Find the mapping that contains this offset
    let mapping = source_map
        .mappings
        .iter()
        .filter(|m| {
            normalize_path(&m.source) == normalize_path(gp_path)
                && m.source_start <= offset
                && offset <= m.source_end
        })
        .min_by_key(|m| m.source_end - m.source_start)?;

    // Read the generated file to convert offset to line:col
    let generated_content = fs::read_to_string(&source_map.generated).ok()?;
    let (go_line, go_col) = line_col(&generated_content, mapping.generated_start);

    Some(GoLocation {
        file: source_map.generated.clone(),
        line: go_line,
        column: go_col,
        offset: mapping.generated_start,
    })
}

/// Given a position in a generated `.go` file, find the corresponding `.gp` position.
pub fn lookup_go_to_gp(
    source_map: &SourceMapFile,
    go_line: usize,
    go_column: usize,
) -> Option<GpLocation> {
    // Read the generated file to convert line:col to offset
    let generated_content = fs::read_to_string(&source_map.generated).ok()?;
    let offset = line_col_to_offset(&generated_content, go_line, go_column)?;

    // Find the mapping that contains this offset
    let mapping = source_map
        .mappings
        .iter()
        .filter(|m| m.generated_start <= offset && offset <= m.generated_end)
        .min_by_key(|m| m.generated_end - m.generated_start)?;

    // Read the source file to convert offset to line:col
    let source_content = fs::read_to_string(&mapping.source).ok()?;
    let (gp_line, gp_col) = line_col(&source_content, mapping.source_start);

    Some(GpLocation {
        file: mapping.source.clone(),
        line: gp_line,
        column: gp_col,
        offset: mapping.source_start,
    })
}

/// Convert line:col (1-indexed) to byte offset.
fn line_col_to_offset(source: &str, target_line: usize, target_col: usize) -> Option<usize> {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in source.char_indices() {
        if line == target_line && col == target_col {
            return Some(idx);
        }
        if ch == '\n' {
            if line == target_line {
                // Past the end of the target line
                return Some(idx);
            }
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    if line == target_line {
        Some(source.len())
    } else {
        None
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_to_offset_basic() {
        let source = "hello\nworld\n";
        assert_eq!(line_col_to_offset(source, 1, 1), Some(0));
        assert_eq!(line_col_to_offset(source, 1, 6), Some(5)); // the \n
        assert_eq!(line_col_to_offset(source, 2, 1), Some(6));
        assert_eq!(line_col_to_offset(source, 2, 3), Some(8));
    }

    #[test]
    fn parse_source_map_json() {
        let json = r#"{
            "version": 1,
            "generated": "test.go",
            "sources": ["test.gp"],
            "mappings": [{
                "source": "test.gp",
                "source_start": 0,
                "source_end": 10,
                "generated_start": 0,
                "generated_end": 20,
                "kind": "function",
                "name": "main"
            }]
        }"#;
        let map = SourceMapFile::from_json(json).expect("parse");
        assert_eq!(map.version, 1);
        assert_eq!(map.mappings.len(), 1);
        assert_eq!(map.mappings[0].name, "main");
    }
}
