use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    ast::{Item, Program},
    codegen::{fmt::format_gp, generate_go},
    diag::Diagnostic,
    parser::parse_program,
    sema::{SemanticModel, analyze_with_model, build_model, lint::lint_program},
};

const GENERATED_GO_FILE_NAME: &str = "zz_goplus_gen.go";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFormat {
    Human,
    Json,
}

pub fn check_file(path: &Path) -> Result<()> {
    check_file_with_format(path, DiagnosticFormat::Human)
}

pub fn check_file_with_format(path: &Path, format: DiagnosticFormat) -> Result<()> {
    let (project, _) = load_analyzed_project_with_format(path, format)?;

    // Strict type validation via Go compiler
    let tmp_dir = std::env::temp_dir().join(format!("goplus_check_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let output = transpile_internal(path, &tmp_dir, true)?;
    write_source_map(&output)?;

    let map_file = output.generated_file.with_extension("go.map");
    let source_map =
        navigate::SourceMapFile::from_json_file(&map_file).context("failed to load source map")?;

    let mut cmd = Command::new("go");
    cmd.current_dir(&output.go_work_dir);
    cmd.arg("build").args(&output.go_args);
    configure_go_command(&mut cmd, &output.package_dir)?;

    let result = cmd.output().context("failed to execute `go build`")?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let mut build_diagnostics = Vec::new();

        for line in stderr.lines() {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() >= 4 {
                let _file = parts[0].trim();
                let msg = parts[3..].join(":");
                if let (Ok(line_num), Ok(col_num)) =
                    (parts[1].parse::<usize>(), parts[2].parse::<usize>())
                    && let Some(loc) = navigate::lookup_go_to_gp(&source_map, line_num, col_num)
                    && let Some(unit) = project.units.iter().find(|u| u.path.ends_with(&loc.file))
                {
                    let byte_offset = compute_byte_offset(&unit.source, loc.line, loc.column);
                    let span = byte_offset..byte_offset;
                    let mut diag = Diagnostic::new(msg.trim(), Some(span));
                    diag = diag.with_code("E0500").with_hint("Go strict type error");
                    build_diagnostics.push((unit.path.clone(), unit.source.clone(), diag));
                }
            }
        }

        if !build_diagnostics.is_empty() {
            let mut grouped: std::collections::HashMap<PathBuf, UnitDiagnostics> =
                std::collections::HashMap::new();
            for (p, src, diag) in build_diagnostics {
                let entry = grouped.entry(p.clone()).or_insert_with(|| UnitDiagnostics {
                    path: p,
                    source: src,
                    diagnostics: Vec::new(),
                });
                entry.diagnostics.push(diag);
            }
            let grouped_vec: Vec<UnitDiagnostics> = grouped.into_values().collect();
            bail!(
                "{}",
                render_unit_diagnostics_with_format(&grouped_vec, format)
            );
        } else {
            // Unmapped error
            bail!("go build failed:\n{}", stderr);
        }
    }

    Ok(())
}

fn compute_byte_offset(source: &str, line: usize, column: usize) -> usize {
    let mut current_line = 1;
    let mut current_byte = 0;
    for (i, c) in source.char_indices() {
        if current_line == line {
            return current_byte + column.saturating_sub(1);
        }
        if c == '\n' {
            current_line += 1;
        }
        current_byte = i + c.len_utf8();
    }
    current_byte
}

pub fn transpile_file(path: &Path, out_dir: &Path) -> Result<()> {
    transpile_file_with_options(path, out_dir, false)
}

pub fn transpile_file_with_options(
    path: &Path,
    out_dir: &Path,
    emit_source_map: bool,
) -> Result<()> {
    let output = transpile_internal(path, out_dir, emit_source_map)?;
    if emit_source_map {
        write_source_map(&output)?;
    }
    println!("generated {}", output.generated_file.display());
    Ok(())
}

pub fn fmt_check_file(path: &Path) -> Result<()> {
    let source_path = project::canonicalize_existing(path)?;
    let source = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read source file {}", source_path.display()))?;
    let program = parse_program(&source).map_err(|diags| {
        let items = vec![UnitDiagnostics {
            path: source_path.clone(),
            source: source.clone(),
            diagnostics: diags,
        }];
        anyhow!(diagnostics::render_unit_diagnostics(&items))
    })?;
    let formatted = format_gp(&program, &source);
    if formatted != source {
        bail!(
            "file {} would be reformatted by `goplus fmt`",
            source_path.display()
        );
    }
    Ok(())
}

pub fn fmt_file(path: &Path) -> Result<()> {
    let source_path = project::canonicalize_existing(path)?;
    let source = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read source file {}", source_path.display()))?;
    let program = parse_program(&source).map_err(|diags| {
        let items = vec![UnitDiagnostics {
            path: source_path.clone(),
            source: source.clone(),
            diagnostics: diags,
        }];
        anyhow!(diagnostics::render_unit_diagnostics(&items))
    })?;
    let formatted = format_gp(&program, &source);
    if formatted == source {
        println!("already formatted {}", source_path.display());
    } else {
        fs::write(&source_path, &formatted)
            .with_context(|| format!("failed to write formatted file {}", source_path.display()))?;
        println!("formatted {}", source_path.display());
    }
    Ok(())
}

pub fn fmt_file_stdout(path: &Path) -> Result<()> {
    let source_path = project::canonicalize_existing(path)?;
    let source = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read source file {}", source_path.display()))?;
    let program = parse_program(&source).map_err(|diags| {
        let items = vec![UnitDiagnostics {
            path: source_path.clone(),
            source: source.clone(),
            diagnostics: diags,
        }];
        anyhow!(diagnostics::render_unit_diagnostics(&items))
    })?;
    let formatted = format_gp(&program, &source);
    print!("{}", formatted);
    Ok(())
}

pub fn lint_file(path: &Path) -> Result<()> {
    lint_file_with_format(path, DiagnosticFormat::Human)
}

pub fn lint_file_with_format(path: &Path, format: DiagnosticFormat) -> Result<()> {
    let (project, model) = load_analyzed_project(path)?;
    let mut all_diagnostics = Vec::new();
    for unit in &project.units {
        let warnings = lint_program(&unit.program, &model);
        if !warnings.is_empty() {
            all_diagnostics.push(UnitDiagnostics {
                path: unit.path.clone(),
                source: unit.source.clone(),
                diagnostics: warnings,
            });
        }
    }
    if all_diagnostics.is_empty() {
        println!("no lint warnings");
    } else {
        let rendered = render_unit_diagnostics_with_format(&all_diagnostics, format);
        println!("{rendered}");
    }
    Ok(())
}

pub fn build_file(path: &Path, out_dir: &Path, out_bin: Option<&Path>) -> Result<()> {
    let output = transpile_internal(path, out_dir, false)?;
    let out_bin = match out_bin {
        Some(path) => resolve_user_path(path)?,
        None => output.package_dir.join(default_binary_name(path)),
    };

    let mut cmd = Command::new("go");
    cmd.current_dir(&output.go_work_dir);
    cmd.arg("build")
        .arg("-o")
        .arg(&out_bin)
        .args(&output.go_args);
    configure_go_command(&mut cmd, &output.package_dir)?;
    let status = cmd.status().context("failed to execute `go build`")?;

    if !status.success() {
        bail!("go build failed for {}", output.package_dir.display());
    }
    println!("built {}", out_bin.display());
    Ok(())
}

pub fn run_file(path: &Path, out_dir: &Path) -> Result<()> {
    let output = transpile_internal(path, out_dir, false)?;
    let mut cmd = Command::new("go");
    cmd.current_dir(&output.go_work_dir);
    cmd.arg("run").args(&output.go_args);
    configure_go_command(&mut cmd, &output.package_dir)?;
    let status = cmd.status().context("failed to execute `go run`")?;

    if !status.success() {
        bail!("go run failed for {}", output.package_dir.display());
    }
    Ok(())
}

pub fn test_file(path: &Path, out_dir: &Path) -> Result<()> {
    let output = transpile_internal(path, out_dir, false)?;
    let mut cmd = Command::new("go");
    cmd.current_dir(&output.go_work_dir);
    cmd.arg("test");
    if output.go_work_dir == output.package_dir {
        cmd.args(collect_go_test_file_args(&output.package_dir)?);
    } else {
        cmd.args(&output.go_args);
    }
    configure_go_command(&mut cmd, &output.package_dir)?;
    let status = cmd.status().context("failed to execute `go test`")?;

    if !status.success() {
        bail!("go test failed for {}", output.package_dir.display());
    }
    Ok(())
}

#[derive(Clone)]
struct SourceUnit {
    path: PathBuf,
    source: String,
    program: Program,
}

#[derive(Clone)]
struct Project {
    source_dir: PathBuf,
    module_root: Option<PathBuf>,
    package_name: String,
    units: Vec<SourceUnit>,
    go_files: Vec<PathBuf>,
}

struct PreparedPackage {
    package_dir: PathBuf,
    generated_file: PathBuf,
    go_work_dir: PathBuf,
    go_args: Vec<OsString>,
    source_map: Option<source_map::SourceMapFile>,
}

fn write_source_map(output: &PreparedPackage) -> Result<()> {
    let map_file = output.generated_file.with_extension("go.map");
    let source_map = output
        .source_map
        .as_ref()
        .context("source map was not prepared")?;
    let content = source_map.to_json()?;
    fs::write(&map_file, content)
        .with_context(|| format!("failed to write source map {}", map_file.display()))?;
    println!("generated {}", map_file.display());
    Ok(())
}

struct UnitDiagnostics {
    path: PathBuf,
    source: String,
    diagnostics: Vec<Diagnostic>,
}

mod diagnostics;
mod emit;
mod go;
mod module;
pub mod navigate;
mod project;
mod source_map;

use diagnostics::render_unit_diagnostics_with_format;
use emit::{emit_project_package, transpile_internal};
use go::{
    collect_go_test_file_args, configure_go_command, default_binary_name,
    resolve_module_go_execution, resolve_non_module_go_execution, resolve_user_path, run_gofmt,
};
use module::{
    copy_go_support_files, read_go_module_name, resolve_output_module_root,
    transpile_module_projects,
};
use project::{
    canonicalize_existing, dir_contains_gp_sources, discover_all_packages, load_analyzed_project,
    load_analyzed_project_with_format, merge_programs, resolve_output_package_dir,
};

pub fn navigate_source_map(
    source_map_path: &Path,
    file: &str,
    line: usize,
    column: usize,
    reverse: bool,
) -> Result<()> {
    let map = navigate::SourceMapFile::from_json_file(source_map_path)?;
    if reverse {
        // Go -> Gp
        match navigate::lookup_go_to_gp(&map, line, column) {
            Some(loc) => {
                println!("{}:{}:{}", loc.file, loc.line, loc.column);
            }
            None => {
                bail!("no source mapping found for {}:{}:{}", file, line, column);
            }
        }
    } else {
        // Gp -> Go
        match navigate::lookup_gp_to_go(&map, file, line, column) {
            Some(loc) => {
                println!("{}:{}:{}", loc.file, loc.line, loc.column);
            }
            None => {
                bail!("no source mapping found for {}:{}:{}", file, line, column);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
