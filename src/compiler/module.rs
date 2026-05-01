use super::*;

pub(super) fn transpile_module_projects(
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
pub(super) fn read_go_module_name(module_root: &Path) -> Result<String> {
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

pub(super) fn resolve_output_module_root(module_root: &Path, out_dir: &Path) -> Result<PathBuf> {
    if out_dir.is_absolute() {
        Ok(out_dir.to_path_buf())
    } else {
        Ok(module_root.join(out_dir))
    }
}
pub(super) fn copy_go_module_metadata(module_root: &Path, module_out_root: &Path) -> Result<()> {
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

pub(super) fn copy_go_files_recursive(
    source_dir: &Path,
    dest_dir: &Path,
    skip_root: &Path,
) -> Result<()> {
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

pub(super) fn copy_go_support_files(go_files: &[PathBuf], package_dir: &Path) -> Result<()> {
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

pub(super) fn collect_local_gp_import_dirs(
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
        let import_path = import.path.as_str();
        let rel_path = if import_path == module_name {
            PathBuf::new()
        } else if let Some(suffix) = import_path.strip_prefix(&format!("{module_name}/")) {
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
