use super::*;

pub(super) fn transpile_internal(path: &Path, out_dir: &Path) -> Result<PreparedPackage> {
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

pub(super) fn emit_project_package(
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
