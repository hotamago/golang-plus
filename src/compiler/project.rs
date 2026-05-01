use super::*;

pub(super) fn load_analyzed_project(path: &Path) -> Result<(Project, SemanticModel)> {
    load_analyzed_project_with_format(path, DiagnosticFormat::Human)
}

pub(super) fn load_analyzed_project_with_format(
    path: &Path,
    format: DiagnosticFormat,
) -> Result<(Project, SemanticModel)> {
    let mut project = load_project_with_format(path, format)?;
    let model = build_model(project.units.iter().map(|unit| &unit.program));

    let mut diagnostics = Vec::new();
    for unit in &mut project.units {
        if let Err(unit_diags) = analyze_with_model(&mut unit.program, &model) {
            diagnostics.push(UnitDiagnostics {
                path: unit.path.clone(),
                source: unit.source.clone(),
                diagnostics: unit_diags,
            });
        }
    }
    if !diagnostics.is_empty() {
        bail!(
            "{}",
            render_unit_diagnostics_with_format(&diagnostics, format)
        );
    }

    Ok((project, model))
}

pub(super) fn load_project_with_format(path: &Path, format: DiagnosticFormat) -> Result<Project> {
    let source_path = canonicalize_existing(path)?;
    let metadata = fs::metadata(&source_path)
        .with_context(|| format!("failed to read metadata for {}", source_path.display()))?;
    if metadata.is_file() && source_path.extension().and_then(|ext| ext.to_str()) != Some("gp") {
        bail!(
            "source must be a `.gp` file or a directory containing `.gp` files: {}",
            source_path.display()
        );
    }

    let source_dir = if metadata.is_dir() {
        source_path.clone()
    } else {
        source_path.parent().map(Path::to_path_buf).ok_or_else(|| {
            anyhow!(
                "source file {} has no parent directory",
                source_path.display()
            )
        })?
    };
    let module_root = find_go_module_root(&source_dir);

    let mut gp_files = Vec::new();
    let mut go_files = Vec::new();
    for entry in fs::read_dir(&source_dir)
        .with_context(|| format!("failed to read source directory {}", source_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!(
                "failed to inspect source directory {}",
                source_dir.display()
            )
        })?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?;
        if !file_type.is_file() {
            continue;
        }

        let file_path = entry.path();
        match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("gp") => gp_files.push(file_path),
            Some("go") => go_files.push(file_path),
            _ => {}
        }
    }

    gp_files.sort();
    go_files.sort();

    if metadata.is_file() && module_root.is_none() && has_multiple_executable_entries(&gp_files)? {
        gp_files = vec![source_path.clone()];
    }

    if gp_files.is_empty() {
        bail!("no `.gp` files found in {}", source_dir.display());
    }

    let mut units = Vec::new();
    let mut diagnostics = Vec::new();
    let mut package_name: Option<String> = None;

    for file_path in gp_files {
        let source = fs::read_to_string(&file_path)
            .with_context(|| format!("failed to read source file {}", file_path.display()))?;
        match parse_program(&source) {
            Ok(program) => {
                if let Some(expected) = &package_name {
                    if program.package != *expected {
                        diagnostics.push(UnitDiagnostics {
                            path: file_path.clone(),
                            source: source.clone(),
                            diagnostics: vec![
                                Diagnostic::new(
                                    format!(
                                        "package `{}` does not match package `{expected}`",
                                        program.package
                                    ),
                                    Some(0..0),
                                )
                                .with_code("E0400")
                                .with_hint(
                                    "all `.gp` files in a package directory must use the same package name",
                                ),
                            ],
                        });
                    }
                } else {
                    package_name = Some(program.package.clone());
                }

                let mut program = program;
                annotate_program_sources(&mut program, &file_path);
                units.push(SourceUnit {
                    path: file_path,
                    source,
                    program,
                });
            }
            Err(unit_diags) => diagnostics.push(UnitDiagnostics {
                path: file_path,
                source,
                diagnostics: unit_diags,
            }),
        }
    }

    if !diagnostics.is_empty() {
        bail!(
            "{}",
            render_unit_diagnostics_with_format(&diagnostics, format)
        );
    }

    Ok(Project {
        source_dir,
        module_root,
        package_name: package_name.unwrap_or_else(|| "main".to_string()),
        units,
        go_files,
    })
}

pub(super) fn merge_programs<'a>(
    package_name: &str,
    programs: impl IntoIterator<Item = &'a Program>,
) -> Program {
    let mut imports = Vec::new();
    let mut items = Vec::new();

    for program in programs {
        imports.extend(program.imports.iter().cloned());
        items.extend(program.items.iter().cloned());
    }

    Program {
        package: package_name.to_string(),
        imports,
        items,
        span: 0..0,
    }
}

fn annotate_program_sources(program: &mut Program, path: &Path) {
    let source = Some(path.display().to_string());
    for item in &mut program.items {
        match item {
            Item::Struct(decl) => decl.source = source.clone(),
            Item::Enum(decl) => decl.source = source.clone(),
            Item::Function(decl) => decl.source = source.clone(),
            Item::Impl(decl) => decl.source = source.clone(),
            Item::Raw(decl) => decl.source = source.clone(),
        }
    }
}

fn has_multiple_executable_entries(paths: &[PathBuf]) -> Result<bool> {
    let mut count = 0;
    for path in paths {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read source file {}", path.display()))?;
        if declares_main_package(&source) && declares_main_function(&source) {
            count += 1;
            if count > 1 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn declares_main_package(source: &str) -> bool {
    source.lines().any(|line| line.trim() == "package main")
}

fn declares_main_function(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim_start().starts_with("fn main("))
}

pub(super) fn canonicalize_existing(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path)
        .with_context(|| format!("failed to access source path {}", path.display()))
}

pub(super) fn find_go_module_root(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if dir.join("go.mod").is_file() {
            return Some(dir.to_path_buf());
        }
    }
    None
}
pub(super) fn resolve_output_package_dir(project: &Project, out_dir: &Path) -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("failed to read current directory")?;
    let current_dir_anchor = fs::canonicalize(&current_dir).unwrap_or_else(|_| current_dir.clone());
    let (out_root, layout_anchor) = if out_dir.is_absolute() {
        (out_dir.to_path_buf(), project.module_root.clone())
    } else if let Some(module_root) = &project.module_root {
        (module_root.join(out_dir), Some(module_root.clone()))
    } else {
        (current_dir.join(out_dir), Some(current_dir_anchor))
    };

    if let Some(anchor) = &layout_anchor
        && let Ok(rel_source_dir) = project.source_dir.strip_prefix(anchor)
    {
        if rel_source_dir.as_os_str().is_empty() {
            return Ok(out_root);
        }
        return Ok(out_root.join(rel_source_dir));
    }

    Ok(out_root)
}
pub(super) fn dir_contains_gp_sources(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }

    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read source directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to inspect {}", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?;
        if file_type.is_file()
            && entry.path().extension().and_then(|ext| ext.to_str()) == Some("gp")
        {
            return Ok(true);
        }
    }

    Ok(false)
}
