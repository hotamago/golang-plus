use std::{
    collections::{BTreeSet, VecDeque},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    ast::Program,
    codegen::generate_go,
    diag::Diagnostic,
    parser::parse_program,
    sema::{SemanticModel, analyze_with_model, build_model},
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
    let _ = load_analyzed_project_with_format(path, format)?;
    Ok(())
}

pub fn transpile_file(path: &Path, out_dir: &Path) -> Result<()> {
    transpile_file_with_options(path, out_dir, false)
}

pub fn transpile_file_with_options(
    path: &Path,
    out_dir: &Path,
    emit_source_map: bool,
) -> Result<()> {
    let output = transpile_internal(path, out_dir)?;
    if emit_source_map {
        write_source_map_stub(&output)?;
    }
    println!("generated {}", output.generated_file.display());
    Ok(())
}

pub fn fmt_check_file(path: &Path) -> Result<()> {
    let _ = load_analyzed_project(path)?;
    Ok(())
}

pub fn build_file(path: &Path, out_dir: &Path, out_bin: Option<&Path>) -> Result<()> {
    let output = transpile_internal(path, out_dir)?;
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
    let output = transpile_internal(path, out_dir)?;
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

struct SourceUnit {
    path: PathBuf,
    source: String,
    program: Program,
}

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
}

fn write_source_map_stub(output: &PreparedPackage) -> Result<()> {
    let map_file = output.generated_file.with_extension("go.map");
    let generated = output
        .generated_file
        .display()
        .to_string()
        .replace('\\', "\\\\");
    let content = format!(
        "{{\n  \"version\": 1,\n  \"generated\": \"{}\",\n  \"mappings\": []\n}}\n",
        generated.replace('"', "\\\"")
    );
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
mod project;

use diagnostics::render_unit_diagnostics_with_format;
use emit::{emit_project_package, transpile_internal};
use go::{
    configure_go_command, default_binary_name, resolve_module_go_execution,
    resolve_non_module_go_execution, resolve_user_path, run_gofmt,
};
use module::{
    copy_go_support_files, read_go_module_name, resolve_output_module_root,
    transpile_module_projects,
};
use project::{
    canonicalize_existing, dir_contains_gp_sources, load_analyzed_project,
    load_analyzed_project_with_format, merge_programs, resolve_output_package_dir,
};

#[cfg(test)]
mod tests;
