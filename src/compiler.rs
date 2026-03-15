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

pub fn check_file(path: &Path) -> Result<()> {
    let _ = load_analyzed_project(path)?;
    Ok(())
}

pub fn transpile_file(path: &Path, out_dir: &Path) -> Result<()> {
    let output = transpile_internal(path, out_dir)?;
    println!("generated {}", output.generated_file.display());
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

struct UnitDiagnostics {
    path: PathBuf,
    source: String,
    diagnostics: Vec<Diagnostic>,
}

fn transpile_internal(path: &Path, out_dir: &Path) -> Result<PreparedPackage> {
    let (project, model) = load_analyzed_project(path)?;

    let (package_dir, generated_file, go_work_dir, go_args) = if let Some(module_root) =
        &project.module_root
    {
        let module_name = read_go_module_name(module_root)?;
        let module_out_root = resolve_output_module_root(module_root, out_dir)?;
        transpile_module_projects(
            &project,
            &model,
            module_root,
            &module_name,
            &module_out_root,
        )?;

        let rel_package_dir = project
            .source_dir
            .strip_prefix(module_root)
            .with_context(|| {
                format!(
                    "failed to resolve package path {} inside module {}",
                    project.source_dir.display(),
                    module_root.display()
                )
            })?;
        let package_dir = if rel_package_dir.as_os_str().is_empty() {
            module_out_root.clone()
        } else {
            module_out_root.join(rel_package_dir)
        };
        let generated_file = package_dir.join(GENERATED_GO_FILE_NAME);
        let (go_work_dir, go_args) = resolve_module_go_execution(&module_out_root, rel_package_dir);
        (package_dir, generated_file, go_work_dir, go_args)
    } else {
        let package_dir = resolve_output_package_dir(&project, out_dir)?;
        let generated_file = emit_project_package(&project, &model, &package_dir, true)?;
        let (go_work_dir, go_args) = resolve_non_module_go_execution(&package_dir)?;
        (package_dir, generated_file, go_work_dir, go_args)
    };

    Ok(PreparedPackage {
        package_dir,
        generated_file,
        go_work_dir,
        go_args,
    })
}

fn emit_project_package(
    project: &Project,
    model: &SemanticModel,
    package_dir: &Path,
    copy_go_files: bool,
) -> Result<PathBuf> {
    let merged_program = merge_programs(
        &project.package_name,
        project.units.iter().map(|unit| &unit.program),
    );
    let generated = generate_go(&merged_program, model);
    fs::create_dir_all(package_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            package_dir.display()
        )
    })?;
    if copy_go_files {
        copy_go_support_files(&project.go_files, package_dir)?;
    }

    let generated_file = package_dir.join(GENERATED_GO_FILE_NAME);
    fs::write(&generated_file, generated).with_context(|| {
        format!(
            "failed to write generated file {}",
            generated_file.display()
        )
    })?;
    run_gofmt(&generated_file)?;
    Ok(generated_file)
}

fn transpile_module_projects(
    entry_project: &Project,
    entry_model: &SemanticModel,
    module_root: &Path,
    module_name: &str,
    module_out_root: &Path,
) -> Result<()> {
    fs::create_dir_all(module_out_root).with_context(|| {
        format!(
            "failed to create module output directory {}",
            module_out_root.display()
        )
    })?;
    copy_go_module_metadata(module_root, module_out_root)?;
    copy_go_files_recursive(module_root, module_out_root, module_out_root)?;

    let mut queue = VecDeque::from([entry_project.source_dir.clone()]);
    let mut visited = BTreeSet::new();

    while let Some(source_dir) = queue.pop_front() {
        if !visited.insert(source_dir.clone()) {
            continue;
        }

        if source_dir == entry_project.source_dir {
            let rel_package_dir = source_dir.strip_prefix(module_root).with_context(|| {
                format!(
                    "failed to resolve package path {} inside module {}",
                    source_dir.display(),
                    module_root.display()
                )
            })?;
            let package_dir = if rel_package_dir.as_os_str().is_empty() {
                module_out_root.to_path_buf()
            } else {
                module_out_root.join(rel_package_dir)
            };
            emit_project_package(entry_project, entry_model, &package_dir, false)?;

            for import_dir in collect_local_gp_import_dirs(entry_project, module_root, module_name)?
            {
                if !visited.contains(&import_dir) {
                    queue.push_back(import_dir);
                }
            }
        } else {
            let (project, model) = load_analyzed_project(&source_dir)?;
            let rel_package_dir = source_dir.strip_prefix(module_root).with_context(|| {
                format!(
                    "failed to resolve package path {} inside module {}",
                    source_dir.display(),
                    module_root.display()
                )
            })?;
            let package_dir = if rel_package_dir.as_os_str().is_empty() {
                module_out_root.to_path_buf()
            } else {
                module_out_root.join(rel_package_dir)
            };
            emit_project_package(&project, &model, &package_dir, false)?;

            for import_dir in collect_local_gp_import_dirs(&project, module_root, module_name)? {
                if !visited.contains(&import_dir) {
                    queue.push_back(import_dir);
                }
            }
        }
    }

    Ok(())
}

fn load_analyzed_project(path: &Path) -> Result<(Project, SemanticModel)> {
    let mut project = load_project(path)?;
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
        bail!("{}", render_unit_diagnostics(&diagnostics));
    }

    Ok((project, model))
}

fn load_project(path: &Path) -> Result<Project> {
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
                                .with_hint(
                                    "all `.gp` files in a package directory must use the same package name",
                                ),
                            ],
                        });
                    }
                } else {
                    package_name = Some(program.package.clone());
                }

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
        bail!("{}", render_unit_diagnostics(&diagnostics));
    }

    Ok(Project {
        source_dir,
        module_root,
        package_name: package_name.unwrap_or_else(|| "main".to_string()),
        units,
        go_files,
    })
}

fn merge_programs<'a>(
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

fn canonicalize_existing(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path)
        .with_context(|| format!("failed to access source path {}", path.display()))
}

fn find_go_module_root(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if dir.join("go.mod").is_file() {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn read_go_module_name(module_root: &Path) -> Result<String> {
    let go_mod = module_root.join("go.mod");
    let content = fs::read_to_string(&go_mod)
        .with_context(|| format!("failed to read module file {}", go_mod.display()))?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if let Some(module_name) = trimmed.strip_prefix("module ") {
            let module_name = module_name.trim();
            if !module_name.is_empty() {
                return Ok(module_name.to_string());
            }
        }
    }

    bail!("failed to parse module path from {}", go_mod.display())
}

fn resolve_output_module_root(module_root: &Path, out_dir: &Path) -> Result<PathBuf> {
    if out_dir.is_absolute() {
        Ok(out_dir.to_path_buf())
    } else {
        Ok(module_root.join(out_dir))
    }
}

fn resolve_output_package_dir(project: &Project, out_dir: &Path) -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("failed to read current directory")?;
    let current_dir_anchor = fs::canonicalize(&current_dir).unwrap_or_else(|_| current_dir.clone());
    let (out_root, layout_anchor) = if out_dir.is_absolute() {
        (out_dir.to_path_buf(), project.module_root.clone())
    } else if let Some(module_root) = &project.module_root {
        (module_root.join(out_dir), Some(module_root.clone()))
    } else {
        (current_dir.join(out_dir), Some(current_dir_anchor))
    };

    if let Some(anchor) = &layout_anchor {
        if let Ok(rel_source_dir) = project.source_dir.strip_prefix(anchor) {
            if rel_source_dir.as_os_str().is_empty() {
                return Ok(out_root);
            }
            return Ok(out_root.join(rel_source_dir));
        }
    }

    Ok(out_root)
}

fn copy_go_module_metadata(module_root: &Path, module_out_root: &Path) -> Result<()> {
    for file_name in ["go.mod", "go.sum"] {
        let source = module_root.join(file_name);
        if !source.is_file() {
            continue;
        }
        let dest = module_out_root.join(file_name);
        fs::copy(&source, &dest).with_context(|| {
            format!(
                "failed to copy module file {} -> {}",
                source.display(),
                dest.display()
            )
        })?;
    }
    Ok(())
}

fn copy_go_files_recursive(source_dir: &Path, dest_dir: &Path, skip_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("failed to read source directory {}", source_dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to inspect {}", source_dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;

        if file_type.is_dir() {
            if path == skip_root
                || path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| matches!(name, ".goplusgen" | ".gocache" | ".gotmp" | ".git"))
                    .unwrap_or(false)
            {
                continue;
            }

            let rel = path.strip_prefix(source_dir).with_context(|| {
                format!(
                    "failed to resolve relative directory {} from {}",
                    path.display(),
                    source_dir.display()
                )
            })?;
            let nested_dest = dest_dir.join(rel);
            fs::create_dir_all(&nested_dest).with_context(|| {
                format!(
                    "failed to create output directory {}",
                    nested_dest.display()
                )
            })?;
            copy_go_files_recursive(&path, &nested_dest, skip_root)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("go") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some(GENERATED_GO_FILE_NAME) {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };
        let dest = dest_dir.join(file_name);
        fs::copy(&path, &dest).with_context(|| {
            format!(
                "failed to copy Go source {} -> {}",
                path.display(),
                dest.display()
            )
        })?;
    }
    Ok(())
}

fn copy_go_support_files(go_files: &[PathBuf], package_dir: &Path) -> Result<()> {
    if go_files
        .iter()
        .any(|path| path.file_name().and_then(|name| name.to_str()) == Some(GENERATED_GO_FILE_NAME))
    {
        bail!(
            "source package already contains reserved generated file name `{}`",
            GENERATED_GO_FILE_NAME
        );
    }

    for source in go_files {
        let Some(file_name) = source.file_name() else {
            continue;
        };
        let dest = package_dir.join(file_name);
        if dest == *source {
            continue;
        }
        fs::copy(source, &dest).with_context(|| {
            format!(
                "failed to copy Go source {} -> {}",
                source.display(),
                dest.display()
            )
        })?;
    }
    Ok(())
}

fn collect_local_gp_import_dirs(
    project: &Project,
    module_root: &Path,
    module_name: &str,
) -> Result<Vec<PathBuf>> {
    let mut dirs = BTreeSet::new();
    for import in project
        .units
        .iter()
        .flat_map(|unit| unit.program.imports.iter())
    {
        let rel_path = if import == module_name {
            PathBuf::new()
        } else if let Some(suffix) = import.strip_prefix(&format!("{module_name}/")) {
            let mut rel = PathBuf::new();
            for segment in suffix.split('/') {
                rel.push(segment);
            }
            rel
        } else {
            continue;
        };

        let import_dir = module_root.join(rel_path);
        if dir_contains_gp_sources(&import_dir)? {
            dirs.insert(canonicalize_existing(&import_dir)?);
        }
    }

    Ok(dirs.into_iter().collect())
}

fn dir_contains_gp_sources(dir: &Path) -> Result<bool> {
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

fn resolve_module_go_execution(
    module_out_root: &Path,
    rel_package_dir: &Path,
) -> (PathBuf, Vec<OsString>) {
    let rel = rel_package_dir.to_string_lossy().replace('\\', "/");
    let target = if rel.is_empty() {
        ".".to_string()
    } else {
        format!("./{}", rel.trim_start_matches("./"))
    };
    (module_out_root.to_path_buf(), vec![OsString::from(target)])
}

fn resolve_non_module_go_execution(package_dir: &Path) -> Result<(PathBuf, Vec<OsString>)> {
    Ok((
        package_dir.to_path_buf(),
        collect_go_file_args(package_dir)?,
    ))
}

fn collect_go_file_args(package_dir: &Path) -> Result<Vec<OsString>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(package_dir).with_context(|| {
        format!(
            "failed to read generated package dir {}",
            package_dir.display()
        )
    })? {
        let entry =
            entry.with_context(|| format!("failed to inspect {}", package_dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?;
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("go") {
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with("_test.go"))
            .unwrap_or(false)
        {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };
        files.push(file_name.to_os_string());
    }

    files.sort();
    if files.is_empty() {
        bail!(
            "no generated Go files found in package directory {}",
            package_dir.display()
        );
    }
    Ok(files)
}

fn resolve_user_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("failed to read current directory")?
            .join(path))
    }
}

fn run_gofmt(file: &Path) -> Result<()> {
    let status = Command::new("gofmt")
        .arg("-w")
        .arg(file)
        .status()
        .context("failed to execute `gofmt` (required for goplus output)")?;

    if !status.success() {
        bail!("gofmt failed on {}", file.display());
    }
    Ok(())
}

fn configure_go_command(cmd: &mut Command, package_dir: &Path) -> Result<()> {
    let package_dir_abs = if package_dir.is_absolute() {
        package_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory")?
            .join(package_dir)
    };
    let cache_dir = package_dir_abs.join(".gocache");
    let tmp_dir = package_dir_abs.join(".gotmp");
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create Go cache dir {}", cache_dir.display()))?;
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("failed to create Go tmp dir {}", tmp_dir.display()))?;
    cmd.env("GOCACHE", cache_dir);
    cmd.env("GOTMPDIR", tmp_dir);
    Ok(())
}

fn render_unit_diagnostics(items: &[UnitDiagnostics]) -> String {
    items
        .iter()
        .flat_map(|item| {
            item.diagnostics
                .iter()
                .map(|diag| diag.render(&item.path.display().to_string(), &item.source))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn default_binary_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .or_else(|| path.file_name().and_then(|name| name.to_str()))
        .unwrap_or("goplus");
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, process::Command};

    use tempfile::tempdir;

    use super::{GENERATED_GO_FILE_NAME, build_file, transpile_file};

    #[test]
    fn transpile_generates_go_file() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("main.gp");
        fs::write(
            &src,
            r#"
package main

fn main() -> ! {
    return
}
"#,
        )
        .expect("write");
        let out_dir = dir.path().join(".goplusgen");
        let result = transpile_file(&src, &out_dir);
        assert!(result.is_ok());
        let generated = out_dir.join(GENERATED_GO_FILE_NAME);
        assert!(generated.exists());
    }

    #[test]
    fn transpile_allows_custom_decorator() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("custom.gp");
        fs::write(
            &src,
            r#"
package main

fn trace(next: func() string, label: string) -> func() string {
    return next
}

@trace("svc")
fn main() -> string {
    return "ok"
}
"#,
        )
        .expect("write");
        let out_dir = dir.path().join(".goplusgen");
        let result = transpile_file(&src, &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn transpile_merges_multi_file_package_and_copies_go_sources() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("main.gp"),
            r#"
package main

fn main() -> ! {
    printMessage()
    return
}
"#,
        )
        .expect("write gp");
        fs::write(
            dir.path().join("helper.gp"),
            r#"
package main

fn printMessage() {
    fmt.Println(messageFromGo())
}
"#,
        )
        .expect("write helper");
        fs::write(
            dir.path().join("bridge.go"),
            r#"
package main

func messageFromGo() string {
    return "hello from go"
}
"#,
        )
        .expect("write go");

        let out_dir = dir.path().join(".goplusgen");
        let result = transpile_file(&dir.path().join("main.gp"), &out_dir);
        assert!(result.is_ok());

        let generated =
            fs::read_to_string(out_dir.join(GENERATED_GO_FILE_NAME)).expect("generated");
        assert!(generated.contains("func mainWarp() error"));
        assert!(generated.contains("func printMessage()"));
        assert!(out_dir.join("bridge.go").exists());
    }

    #[test]
    fn transpile_mirrors_module_relative_output_layout() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("go.mod"),
            "module example.com/demo\n\ngo 1.23.0\n",
        )
        .expect("write go.mod");
        let src_dir = dir.path().join("cmd").join("demo");
        fs::create_dir_all(&src_dir).expect("mkdir");
        fs::write(
            src_dir.join("main.gp"),
            r#"
package main

fn main() -> ! {
    return
}
"#,
        )
        .expect("write gp");

        let out_root = dir.path().join(".goplusgen");
        let result = transpile_file(&src_dir.join("main.gp"), &out_root);
        assert!(result.is_ok());
        assert!(
            out_root
                .join("cmd")
                .join("demo")
                .join(GENERATED_GO_FILE_NAME)
                .exists()
        );
    }

    #[test]
    fn build_links_gp_and_go_sources() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("main.gp"),
            r#"
package main

import "fmt"

fn main() -> ! {
    fmt.Println(fromGo())
    return
}
"#,
        )
        .expect("write main");
        fs::write(
            dir.path().join("helper.gp"),
            r#"
package main

fn fromGp() -> string {
    return "world"
}
"#,
        )
        .expect("write helper");
        fs::write(
            dir.path().join("bridge.go"),
            r#"
package main

func fromGo() string {
    return "hello " + fromGp()
}
"#,
        )
        .expect("write bridge");

        let out_dir = dir.path().join(".goplusgen");
        let out_bin = dir
            .path()
            .join(if cfg!(windows) { "demo.exe" } else { "demo" });
        let result = build_file(&dir.path().join("main.gp"), &out_dir, Some(&out_bin));
        assert!(result.is_ok());
        assert!(out_bin.exists());
    }

    #[test]
    fn build_nested_module_package_with_imports_and_mixed_sources() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("go.mod"),
            "module example.com/demo\n\ngo 1.23.0\n",
        )
        .expect("write go.mod");

        let app_dir = dir.path().join("cmd").join("demo");
        let support_dir = dir.path().join("pkg").join("support");
        let label_dir = dir.path().join("internal").join("label");
        fs::create_dir_all(&app_dir).expect("mkdir app");
        fs::create_dir_all(&support_dir).expect("mkdir support");
        fs::create_dir_all(&label_dir).expect("mkdir label");

        fs::write(
            app_dir.join("main.gp"),
            r#"
package main

import "fmt"
import "example.com/demo/pkg/support"

fn main() -> ! {
    fmt.Println(buildBanner("goplus"))
    fmt.Println(callFromGoFile())
    fmt.Println(support.FromNestedPackages())
    return
}
"#,
        )
        .expect("write main");
        fs::write(
            app_dir.join("messages.gp"),
            r#"
package main

fn buildBanner(name: string) -> string {
    return "[same package gp] hello, " + name
}

fn fromGpFile() -> string {
    return "same-folder .gp files share one package namespace"
}
"#,
        )
        .expect("write messages");
        fs::write(
            app_dir.join("bridge.go"),
            r#"
package main

func callFromGoFile() string {
    return "[same package go] " + fromGpFile()
}
"#,
        )
        .expect("write bridge");
        fs::write(
            support_dir.join("support.gp"),
            r#"
package support

import "example.com/demo/internal/label"

fn FromNestedPackages() -> string {
    return label.Prefix() + " support.FromNestedPackages()"
}
"#,
        )
        .expect("write support");
        fs::write(
            label_dir.join("label.gp"),
            r#"
package label

fn Prefix() -> string {
    return "[imported package]"
}
"#,
        )
        .expect("write label");

        let out_root = dir.path().join(".goplusgen");
        let out_bin = dir.path().join(if cfg!(windows) {
            "nested.exe"
        } else {
            "nested"
        });
        let result = build_file(&app_dir.join("main.gp"), &out_root, Some(&out_bin));
        assert!(result.is_ok());
        assert!(
            out_root
                .join("cmd")
                .join("demo")
                .join(GENERATED_GO_FILE_NAME)
                .exists()
        );
        assert!(
            out_root
                .join("pkg")
                .join("support")
                .join(GENERATED_GO_FILE_NAME)
                .exists()
        );
        assert!(
            out_root
                .join("internal")
                .join("label")
                .join(GENERATED_GO_FILE_NAME)
                .exists()
        );
        assert!(out_bin.exists());

        let output = Command::new(&out_bin).output().expect("run binary");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("[same package gp] hello, goplus"));
        assert!(
            stdout.contains("[same package go] same-folder .gp files share one package namespace")
        );
        assert!(stdout.contains("[imported package] support.FromNestedPackages()"));
    }

    #[test]
    fn default_binary_is_platform_specific() {
        let name = super::default_binary_name(&PathBuf::from("demo.gp"));
        if cfg!(windows) {
            assert!(name.ends_with(".exe"));
        } else {
            assert_eq!(name, "demo");
        }
    }
}
